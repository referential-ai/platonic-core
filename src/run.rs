//! Pure run state machine.

use crate::{
    ContextPack, Error, HarnessEvent, PolicyDecision, RecordedEvent, RunId, ToolCall, ToolCallId,
    ToolProposal, TurnId,
};

/// Host command requested by the run state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum RunCommand {
    /// Ask the host to make a model request.
    RequestModel {
        turn_id: TurnId,
        step: u32,
        context: ContextPack,
    },
    /// Ask the host to obtain approval for a tool call.
    AwaitApproval { call_id: ToolCallId, reason: String },
    /// Ask the host to execute a validated tool call.
    ExecuteTool { call: ToolCall },
}

/// Current phase of one run.
#[derive(Clone, Debug, PartialEq)]
pub enum RunPhase {
    /// No `run_started` event has been applied.
    NotStarted,
    /// The run is waiting for context to be built for the next model turn.
    ReadyForContext,
    /// Context is built and the next command is a model request.
    ReadyToRequestModel {
        turn_id: TurnId,
        step: u32,
        context: ContextPack,
    },
    /// A model request was recorded; the run is waiting for the response.
    AwaitingModelResponse { turn_id: TurnId, step: u32 },
    /// The model response contained at least one tool proposal.
    AwaitingToolCall {
        turn_id: TurnId,
        proposals: Vec<ToolProposal>,
    },
    /// A validated tool call is waiting for policy evaluation.
    AwaitingPolicy { call: ToolCall },
    /// Policy requires approval before tool execution.
    AwaitingApproval { call: ToolCall, reason: String },
    /// Policy denied the tool call.
    PolicyDenied { call_id: ToolCallId, reason: String },
    /// A human or host actor denied the pending approval.
    ApprovalDenied { call_id: ToolCallId, reason: String },
    /// The tool call may be executed.
    ReadyToExecuteTool { call: ToolCall },
    /// Tool execution has started.
    ToolRunning { call_id: ToolCallId },
    /// A tool failed and the run should be failed.
    ToolFailed { call_id: ToolCallId, reason: String },
    /// The run may be finished.
    ReadyToFinish,
    /// The run finished successfully.
    Finished,
    /// The run finished unsuccessfully.
    Failed { reason: String },
}

/// Durable state for one run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunState {
    run_id: Option<RunId>,
    next_seq: u64,
    next_model_step: u32,
    phase: RunPhase,
}

impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}

impl RunState {
    /// Creates an empty run state.
    pub fn new() -> Self {
        Self {
            run_id: None,
            next_seq: 0,
            next_model_step: 0,
            phase: RunPhase::NotStarted,
        }
    }

    /// Returns the run id after `run_started` has been applied.
    pub fn run_id(&self) -> Option<&RunId> {
        self.run_id.as_ref()
    }

    /// Returns the next expected event sequence number.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// Returns the current run phase.
    pub fn phase(&self) -> &RunPhase {
        &self.phase
    }

    /// Returns the pending host command, if this state needs host IO.
    pub fn pending_command(&self) -> Option<RunCommand> {
        match &self.phase {
            RunPhase::ReadyToRequestModel {
                turn_id,
                step,
                context,
            } => Some(RunCommand::RequestModel {
                turn_id: turn_id.clone(),
                step: *step,
                context: context.clone(),
            }),
            RunPhase::AwaitingApproval { call, reason } => Some(RunCommand::AwaitApproval {
                call_id: call.id.clone(),
                reason: reason.clone(),
            }),
            RunPhase::ReadyToExecuteTool { call } => {
                Some(RunCommand::ExecuteTool { call: call.clone() })
            }
            _ => None,
        }
    }

    /// Applies one recorded event.
    pub fn apply(&mut self, record: &RecordedEvent) -> Result<(), Error> {
        if record.seq != self.next_seq {
            return Err(Error::SequenceMismatch {
                expected: self.next_seq,
                actual: record.seq,
            });
        }

        if let Some(expected) = &self.run_id {
            let actual = record.event.run_id();
            if actual != expected {
                return Err(Error::RunIdMismatch {
                    expected: expected.to_string(),
                    actual: actual.to_string(),
                });
            }
        }

        self.apply_event(&record.event)?;
        self.next_seq += 1;
        Ok(())
    }

    fn apply_event(&mut self, event: &HarnessEvent) -> Result<(), Error> {
        match (&self.phase, event) {
            (RunPhase::NotStarted, HarnessEvent::RunStarted { run_id, .. }) => {
                self.run_id = Some(run_id.clone());
                self.phase = RunPhase::ReadyForContext;
                Ok(())
            }
            (
                RunPhase::ReadyForContext,
                HarnessEvent::ContextBuilt {
                    turn_id, context, ..
                },
            ) => {
                context.validate_budget()?;
                self.phase = RunPhase::ReadyToRequestModel {
                    turn_id: turn_id.clone(),
                    step: self.next_model_step,
                    context: context.clone(),
                };
                Ok(())
            }
            (
                RunPhase::ReadyToRequestModel {
                    turn_id,
                    step,
                    context: _,
                },
                HarnessEvent::ModelRequested {
                    turn_id: actual_turn_id,
                    step: actual_step,
                    ..
                },
            ) => {
                ensure_turn(turn_id, actual_turn_id)?;
                ensure_step(*step, *actual_step)?;
                self.phase = RunPhase::AwaitingModelResponse {
                    turn_id: turn_id.clone(),
                    step: *step,
                };
                Ok(())
            }
            (
                RunPhase::AwaitingModelResponse { turn_id, step },
                HarnessEvent::ModelResponded {
                    turn_id: actual_turn_id,
                    step: actual_step,
                    proposed_calls,
                    ..
                },
            ) => {
                ensure_turn(turn_id, actual_turn_id)?;
                ensure_step(*step, *actual_step)?;
                self.next_model_step += 1;
                self.phase = if proposed_calls.is_empty() {
                    RunPhase::ReadyToFinish
                } else {
                    RunPhase::AwaitingToolCall {
                        turn_id: turn_id.clone(),
                        proposals: proposed_calls.clone(),
                    }
                };
                Ok(())
            }
            (
                RunPhase::AwaitingToolCall { turn_id, proposals },
                HarnessEvent::ToolCallProposed {
                    turn_id: actual_turn_id,
                    call,
                    ..
                },
            ) => {
                ensure_turn(turn_id, actual_turn_id)?;
                ensure_proposed(proposals, call)?;
                self.phase = RunPhase::AwaitingPolicy { call: call.clone() };
                Ok(())
            }
            (
                RunPhase::AwaitingPolicy { call },
                HarnessEvent::PolicyEvaluated {
                    call_id, decision, ..
                },
            ) => {
                ensure_call(&call.id, call_id)?;
                match decision {
                    PolicyDecision::Allow => {
                        self.phase = RunPhase::ReadyToExecuteTool { call: call.clone() };
                    }
                    PolicyDecision::RequireApproval { reason } => {
                        self.phase = RunPhase::AwaitingApproval {
                            call: call.clone(),
                            reason: reason.clone(),
                        };
                    }
                    PolicyDecision::Deny { reason } => {
                        self.phase = RunPhase::PolicyDenied {
                            call_id: call.id.clone(),
                            reason: reason.clone(),
                        };
                    }
                }
                Ok(())
            }
            (
                RunPhase::AwaitingApproval { call, .. },
                HarnessEvent::ApprovalGranted { call_id, .. },
            ) => {
                ensure_call(&call.id, call_id)?;
                self.phase = RunPhase::ReadyToExecuteTool { call: call.clone() };
                Ok(())
            }
            (
                RunPhase::AwaitingApproval { call, .. },
                HarnessEvent::ApprovalDenied {
                    call_id, reason, ..
                },
            ) => {
                ensure_call(&call.id, call_id)?;
                self.phase = RunPhase::ApprovalDenied {
                    call_id: call.id.clone(),
                    reason: reason.clone(),
                };
                Ok(())
            }
            (RunPhase::ReadyToExecuteTool { call }, HarnessEvent::ToolStarted { call_id, .. }) => {
                ensure_call(&call.id, call_id)?;
                self.phase = RunPhase::ToolRunning {
                    call_id: call.id.clone(),
                };
                Ok(())
            }
            (RunPhase::ToolRunning { call_id }, HarnessEvent::ToolFinished { result, .. }) => {
                ensure_call(call_id, &result.call_id)?;
                self.phase = RunPhase::ReadyToFinish;
                Ok(())
            }
            (
                RunPhase::ToolRunning { call_id },
                HarnessEvent::ToolFailed {
                    call_id: actual_call_id,
                    reason,
                    ..
                },
            ) => {
                ensure_call(call_id, actual_call_id)?;
                self.phase = RunPhase::ToolFailed {
                    call_id: call_id.clone(),
                    reason: reason.clone(),
                };
                Ok(())
            }
            (RunPhase::ReadyToFinish, HarnessEvent::RunFinished { .. }) => {
                self.phase = RunPhase::Finished;
                Ok(())
            }
            (phase, HarnessEvent::RunFailed { reason, .. }) if phase.can_fail() => {
                self.phase = RunPhase::Failed {
                    reason: reason.clone(),
                };
                Ok(())
            }
            (RunPhase::Finished | RunPhase::Failed { .. }, _) => Err(invalid(&self.phase, event)),
            _ => Err(invalid(&self.phase, event)),
        }
    }
}

impl RunPhase {
    fn name(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::ReadyForContext => "ready_for_context",
            Self::ReadyToRequestModel { .. } => "ready_to_request_model",
            Self::AwaitingModelResponse { .. } => "awaiting_model_response",
            Self::AwaitingToolCall { .. } => "awaiting_tool_call",
            Self::AwaitingPolicy { .. } => "awaiting_policy",
            Self::AwaitingApproval { .. } => "awaiting_approval",
            Self::PolicyDenied { .. } => "policy_denied",
            Self::ApprovalDenied { .. } => "approval_denied",
            Self::ReadyToExecuteTool { .. } => "ready_to_execute_tool",
            Self::ToolRunning { .. } => "tool_running",
            Self::ToolFailed { .. } => "tool_failed",
            Self::ReadyToFinish => "ready_to_finish",
            Self::Finished => "finished",
            Self::Failed { .. } => "failed",
        }
    }

    fn can_fail(&self) -> bool {
        !matches!(
            self,
            Self::NotStarted | Self::Finished | Self::Failed { .. }
        )
    }
}

fn ensure_turn(expected: &TurnId, actual: &TurnId) -> Result<(), Error> {
    if expected == actual {
        return Ok(());
    }
    Err(Error::TurnMismatch {
        expected: expected.to_string(),
        actual: actual.to_string(),
    })
}

fn ensure_step(expected: u32, actual: u32) -> Result<(), Error> {
    if expected == actual {
        return Ok(());
    }
    Err(Error::StepMismatch { expected, actual })
}

fn ensure_call(expected: &ToolCallId, actual: &ToolCallId) -> Result<(), Error> {
    if expected == actual {
        return Ok(());
    }
    Err(Error::ToolCallMismatch {
        expected: expected.to_string(),
        actual: actual.to_string(),
    })
}

fn ensure_proposed(proposals: &[ToolProposal], call: &ToolCall) -> Result<(), Error> {
    if proposals
        .iter()
        .any(|proposal| proposal.tool == call.tool && proposal.input == call.input)
    {
        return Ok(());
    }
    Err(Error::UnproposedToolCall)
}

fn invalid(phase: &RunPhase, event: &HarnessEvent) -> Error {
    Error::InvalidTransition {
        phase: phase.name(),
        event: event.name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActorId, AgentId, ContextFragment, ContextLane, EffectClass, Message, MessageRole,
        ModelName, ModelUsage, ResultVisibility, ToolName, ToolProposal, ToolResult,
    };
    use serde_json::json;

    fn run_id() -> RunId {
        RunId::new("run_1").unwrap()
    }

    fn other_run_id() -> RunId {
        RunId::new("run_2").unwrap()
    }

    fn agent_id() -> AgentId {
        AgentId::new("agent_1").unwrap()
    }

    fn turn_id() -> TurnId {
        TurnId::new("turn_1").unwrap()
    }

    fn other_turn_id() -> TurnId {
        TurnId::new("turn_2").unwrap()
    }

    fn call_id() -> ToolCallId {
        ToolCallId::new("call_1").unwrap()
    }

    fn actor_id() -> ActorId {
        ActorId::new("human_1").unwrap()
    }

    fn context(tokens: u32) -> ContextPack {
        ContextPack {
            token_budget: 100,
            fragments: vec![ContextFragment {
                lane: ContextLane::CurrentTask,
                source: "user".into(),
                content: "read README".into(),
                estimated_tokens: tokens,
            }],
        }
    }

    fn proposal() -> ToolProposal {
        ToolProposal {
            tool: ToolName::new("file.read").unwrap(),
            input: json!({ "path": "README.md" }),
        }
    }

    fn call(effect: EffectClass) -> ToolCall {
        ToolCall {
            id: call_id(),
            tool: ToolName::new("file.read").unwrap(),
            effect,
            input: json!({ "path": "README.md" }),
        }
    }

    fn result() -> ToolResult {
        ToolResult {
            call_id: call_id(),
            summary: "read README".into(),
            data: json!({ "bytes": 123 }),
            artifacts: vec![],
            visibility: ResultVisibility::Both,
        }
    }

    fn usage() -> ModelUsage {
        ModelUsage {
            input_tokens: 50,
            output_tokens: 10,
        }
    }

    fn rec(seq: u64, event: HarnessEvent) -> RecordedEvent {
        RecordedEvent {
            seq,
            occurred_at_ms: 1_700_000_000_000 + seq,
            event,
        }
    }

    fn start_event(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::RunStarted {
                run_id: run_id(),
                agent_id: agent_id(),
            },
        )
    }

    fn context_event(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ContextBuilt {
                run_id: run_id(),
                turn_id: turn_id(),
                context: context(10),
            },
        )
    }

    fn model_requested(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ModelRequested {
                run_id: run_id(),
                turn_id: turn_id(),
                step: 0,
                model: ModelName::new("claude-fable-5").unwrap(),
            },
        )
    }

    fn model_responded(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ModelResponded {
                run_id: run_id(),
                turn_id: turn_id(),
                step: 0,
                output: Message {
                    role: MessageRole::Assistant,
                    content: "I should read the file.".into(),
                },
                proposed_calls: vec![proposal()],
                usage: usage(),
            },
        )
    }

    fn tool_proposed(seq: u64, effect: EffectClass) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ToolCallProposed {
                run_id: run_id(),
                turn_id: turn_id(),
                call: call(effect),
            },
        )
    }

    fn unproposed_tool(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ToolCallProposed {
                run_id: run_id(),
                turn_id: turn_id(),
                call: ToolCall {
                    id: call_id(),
                    tool: ToolName::new("file.write").unwrap(),
                    effect: EffectClass::WorkspaceWrite,
                    input: json!({ "path": "README.md", "content": "surprise" }),
                },
            },
        )
    }

    fn require_approval(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::PolicyEvaluated {
                run_id: run_id(),
                call_id: call_id(),
                decision: PolicyDecision::RequireApproval {
                    reason: "workspace write needs approval".into(),
                },
            },
        )
    }

    fn deny_policy(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::PolicyEvaluated {
                run_id: run_id(),
                call_id: call_id(),
                decision: PolicyDecision::Deny {
                    reason: "policy denied".into(),
                },
            },
        )
    }

    fn approval_granted(seq: u64) -> RecordedEvent {
        rec(
            seq,
            HarnessEvent::ApprovalGranted {
                run_id: run_id(),
                call_id: call_id(),
                actor_id: actor_id(),
            },
        )
    }

    fn base_until_approval_required() -> Vec<RecordedEvent> {
        vec![
            start_event(0),
            context_event(1),
            model_requested(2),
            model_responded(3),
            tool_proposed(4, EffectClass::WorkspaceWrite),
            require_approval(5),
        ]
    }

    fn apply_all(events: &[RecordedEvent]) -> RunState {
        let mut state = RunState::new();
        for event in events {
            state.apply(event).unwrap();
        }
        state
    }

    #[test]
    fn happy_path_emits_expected_commands_and_finishes() {
        let events = vec![
            start_event(0),
            context_event(1),
            model_requested(2),
            model_responded(3),
            tool_proposed(4, EffectClass::WorkspaceWrite),
            require_approval(5),
            approval_granted(6),
            rec(
                7,
                HarnessEvent::ToolStarted {
                    run_id: run_id(),
                    call_id: call_id(),
                },
            ),
            rec(
                8,
                HarnessEvent::ToolFinished {
                    run_id: run_id(),
                    result: result(),
                },
            ),
            rec(9, HarnessEvent::RunFinished { run_id: run_id() }),
        ];

        let mut state = RunState::new();
        state.apply(&events[0]).unwrap();
        state.apply(&events[1]).unwrap();
        assert!(matches!(
            state.pending_command(),
            Some(RunCommand::RequestModel { step: 0, .. })
        ));
        state.apply(&events[2]).unwrap();
        state.apply(&events[3]).unwrap();
        state.apply(&events[4]).unwrap();
        state.apply(&events[5]).unwrap();
        assert!(matches!(
            state.pending_command(),
            Some(RunCommand::AwaitApproval { .. })
        ));
        state.apply(&events[6]).unwrap();
        assert!(matches!(
            state.pending_command(),
            Some(RunCommand::ExecuteTool { .. })
        ));
        for event in &events[7..] {
            state.apply(event).unwrap();
        }

        assert_eq!(state.phase(), &RunPhase::Finished);
        assert_eq!(state.next_seq(), 10);
        assert_eq!(apply_all(&events), state);
    }

    #[test]
    fn approval_denial_never_emits_execute_tool() {
        let mut events = base_until_approval_required();
        events.push(rec(
            6,
            HarnessEvent::ApprovalDenied {
                run_id: run_id(),
                call_id: call_id(),
                actor_id: actor_id(),
                reason: "no".into(),
            },
        ));
        events.push(rec(
            7,
            HarnessEvent::RunFailed {
                run_id: run_id(),
                reason: "approval denied".into(),
            },
        ));

        let mut state = RunState::new();
        for event in &events[..7] {
            state.apply(event).unwrap();
        }
        assert!(state.pending_command().is_none());
        assert!(matches!(state.phase(), RunPhase::ApprovalDenied { .. }));
        state.apply(&events[7]).unwrap();
        assert!(matches!(state.phase(), RunPhase::Failed { .. }));
    }

    #[test]
    fn policy_denial_never_emits_execute_tool() {
        let events = [
            start_event(0),
            context_event(1),
            model_requested(2),
            model_responded(3),
            tool_proposed(4, EffectClass::SecretAccess),
            deny_policy(5),
            rec(
                6,
                HarnessEvent::RunFailed {
                    run_id: run_id(),
                    reason: "policy denied".into(),
                },
            ),
        ];

        let mut state = RunState::new();
        for event in &events[..6] {
            state.apply(event).unwrap();
        }
        assert!(state.pending_command().is_none());
        assert!(matches!(state.phase(), RunPhase::PolicyDenied { .. }));
        state.apply(&events[6]).unwrap();
        assert!(matches!(state.phase(), RunPhase::Failed { .. }));
    }

    #[test]
    fn tool_failure_records_failure_before_run_failure() {
        let mut events = base_until_approval_required();
        events.push(approval_granted(6));
        events.push(rec(
            7,
            HarnessEvent::ToolStarted {
                run_id: run_id(),
                call_id: call_id(),
            },
        ));
        events.push(rec(
            8,
            HarnessEvent::ToolFailed {
                run_id: run_id(),
                call_id: call_id(),
                reason: "tool crashed".into(),
            },
        ));
        events.push(rec(
            9,
            HarnessEvent::RunFailed {
                run_id: run_id(),
                reason: "tool crashed".into(),
            },
        ));

        let state = apply_all(&events);
        assert!(matches!(state.phase(), RunPhase::Failed { .. }));
    }

    #[test]
    fn out_of_order_sequences_are_rejected() {
        let mut state = RunState::new();
        let err = state.apply(&start_event(1)).unwrap_err();
        assert_eq!(
            err,
            Error::SequenceMismatch {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn duplicate_sequences_are_rejected() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        let err = state.apply(&context_event(0)).unwrap_err();
        assert_eq!(
            err,
            Error::SequenceMismatch {
                expected: 1,
                actual: 0
            }
        );
    }

    #[test]
    fn run_id_mismatches_are_rejected() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        let err = state
            .apply(&rec(
                1,
                HarnessEvent::ContextBuilt {
                    run_id: other_run_id(),
                    turn_id: turn_id(),
                    context: context(10),
                },
            ))
            .unwrap_err();
        assert_eq!(
            err,
            Error::RunIdMismatch {
                expected: "run_1".into(),
                actual: "run_2".into()
            }
        );
    }

    #[test]
    fn step_mismatches_are_rejected() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        state.apply(&context_event(1)).unwrap();

        let err = state
            .apply(&rec(
                2,
                HarnessEvent::ModelRequested {
                    run_id: run_id(),
                    turn_id: turn_id(),
                    step: 1,
                    model: ModelName::new("claude-fable-5").unwrap(),
                },
            ))
            .unwrap_err();
        assert_eq!(
            err,
            Error::StepMismatch {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn turn_mismatches_are_rejected() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        state.apply(&context_event(1)).unwrap();

        let err = state
            .apply(&rec(
                2,
                HarnessEvent::ModelRequested {
                    run_id: run_id(),
                    turn_id: other_turn_id(),
                    step: 0,
                    model: ModelName::new("claude-fable-5").unwrap(),
                },
            ))
            .unwrap_err();
        assert_eq!(
            err,
            Error::TurnMismatch {
                expected: "turn_1".into(),
                actual: "turn_2".into()
            }
        );
    }

    #[test]
    fn illegal_event_order_is_rejected() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        let err = state.apply(&model_requested(1)).unwrap_err();
        assert_eq!(
            err,
            Error::InvalidTransition {
                phase: "ready_for_context",
                event: "model_requested"
            }
        );
    }

    #[test]
    fn tool_call_must_match_model_proposal() {
        let mut state = RunState::new();
        for event in &[
            start_event(0),
            context_event(1),
            model_requested(2),
            model_responded(3),
        ] {
            state.apply(event).unwrap();
        }

        let err = state.apply(&unproposed_tool(4));
        assert_eq!(err, Err(Error::UnproposedToolCall));
    }

    #[test]
    fn terminal_runs_emit_no_commands_and_reject_more_events() {
        let mut events = base_until_approval_required();
        events.push(approval_granted(6));
        events.push(rec(
            7,
            HarnessEvent::ToolStarted {
                run_id: run_id(),
                call_id: call_id(),
            },
        ));
        events.push(rec(
            8,
            HarnessEvent::ToolFinished {
                run_id: run_id(),
                result: result(),
            },
        ));
        events.push(rec(9, HarnessEvent::RunFinished { run_id: run_id() }));

        let mut state = apply_all(&events);
        assert!(state.pending_command().is_none());
        let err = state.apply(&rec(10, HarnessEvent::RunFinished { run_id: run_id() }));
        assert!(matches!(err, Err(Error::InvalidTransition { .. })));
    }

    #[test]
    fn run_failed_closes_any_started_non_terminal_phase() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        state.apply(&context_event(1)).unwrap();
        state.apply(&model_requested(2)).unwrap();

        state
            .apply(&rec(
                3,
                HarnessEvent::RunFailed {
                    run_id: run_id(),
                    reason: "model request failed".into(),
                },
            ))
            .unwrap();

        assert!(matches!(state.phase(), RunPhase::Failed { .. }));
        assert!(state.pending_command().is_none());
    }

    #[test]
    fn context_budget_is_validated_before_model_request() {
        let mut state = RunState::new();
        state.apply(&start_event(0)).unwrap();
        let over_budget = rec(
            1,
            HarnessEvent::ContextBuilt {
                run_id: run_id(),
                turn_id: turn_id(),
                context: ContextPack {
                    token_budget: 5,
                    fragments: context(10).fragments,
                },
            },
        );

        assert!(matches!(
            state.apply(&over_budget),
            Err(Error::ContextBudgetExceeded {
                used: 10,
                budget: 5
            })
        ));
        assert!(state.pending_command().is_none());
    }
}
