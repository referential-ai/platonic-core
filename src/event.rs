//! Durable harness event ledger types.

use crate::{
    ActorId, AgentId, ContextPack, Message, ModelName, PolicyDecision, RunId, ToolCall, ToolCallId,
    ToolProposal, ToolResult, TurnId,
};
use serde::{Deserialize, Serialize};

/// Token usage reported by a model provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelUsage {
    /// Prompt-side tokens used by the request.
    pub input_tokens: u32,
    /// Completion-side tokens emitted by the model.
    pub output_tokens: u32,
}

/// One durable event with host-supplied ordering metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedEvent {
    /// Per-run sequence number, contiguous from zero.
    pub seq: u64,
    /// Host-supplied wall-clock timestamp in milliseconds since Unix epoch.
    pub occurred_at_ms: u64,
    /// Recorded run fact.
    pub event: HarnessEvent,
}

/// Durable event log entries. Transcript, metrics, replay, and audit views are derived from this log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum HarnessEvent {
    /// Binds a fresh state machine to one run and agent.
    RunStarted {
        /// Durable run identifier shared by every later event.
        run_id: RunId,
        /// Agent identity selected by the host for this run.
        agent_id: AgentId,
    },
    /// Records the exact bounded context selected for a model turn.
    ContextBuilt {
        /// Run receiving the context.
        run_id: RunId,
        /// New turn identifier; concluded turn identifiers cannot be reused.
        turn_id: TurnId,
        /// Lane-labeled context that must pass budget validation.
        context: ContextPack,
    },
    /// Records that a model request crossed the provider boundary.
    ModelRequested {
        /// Run making the request.
        run_id: RunId,
        /// Turn whose context is being submitted.
        turn_id: TurnId,
        /// Monotonic model step expected by the run state.
        step: u32,
        /// Host-selected model used for the request.
        model: ModelName,
    },
    /// Records the normalized result returned by the pending model request.
    ModelResponded {
        /// Run receiving the response.
        run_id: RunId,
        /// Turn of the pending model request.
        turn_id: TurnId,
        /// Model step of the pending request.
        step: u32,
        /// Normalized model-authored message.
        output: Message,
        /// Unvalidated model proposals; an empty list concludes the turn.
        proposed_calls: Vec<ToolProposal>,
        /// Provider-reported token usage for the response.
        usage: ModelUsage,
    },
    /// Records a model proposal after host validation and effect classification.
    ToolCallProposed {
        /// Run containing the proposal.
        run_id: RunId,
        /// Turn that produced the proposal.
        turn_id: TurnId,
        /// Host-validated call; its tool and input must match a pending proposal.
        call: ToolCall,
    },
    /// Records the policy decision for the pending validated call.
    PolicyEvaluated {
        /// Run containing the call.
        run_id: RunId,
        /// Pending call evaluated by policy.
        call_id: ToolCallId,
        /// Durable allow, approval, or denial decision.
        decision: PolicyDecision,
    },
    /// Records who granted a pending approval.
    ApprovalGranted {
        /// Run containing the call.
        run_id: RunId,
        /// Pending call approved for execution.
        call_id: ToolCallId,
        /// Human or host actor that granted approval.
        actor_id: ActorId,
    },
    /// Records who denied a pending approval and why.
    ApprovalDenied {
        /// Run containing the call.
        run_id: RunId,
        /// Pending call denied before execution.
        call_id: ToolCallId,
        /// Human or host actor that denied approval.
        actor_id: ActorId,
        /// Durable denial reason for audit and continuation context.
        reason: String,
    },
    /// Records that the host began executing the approved call.
    ToolStarted {
        /// Run containing the call.
        run_id: RunId,
        /// Approved call that crossed the execution boundary.
        call_id: ToolCallId,
    },
    /// Records the structured result returned by the running call.
    ToolFinished {
        /// Run containing the call.
        run_id: RunId,
        /// Result whose call id must match the running call.
        result: ToolResult,
    },
    /// Records that the running call failed without a result.
    ToolFailed {
        /// Run containing the call.
        run_id: RunId,
        /// Running call that failed.
        call_id: ToolCallId,
        /// Durable host-reported failure reason.
        reason: String,
    },
    /// Terminates a concluded turn as a successful run.
    RunFinished {
        /// Run that completed.
        run_id: RunId,
    },
    /// Terminates any started, nonterminal run as failed.
    RunFailed {
        /// Run that failed.
        run_id: RunId,
        /// Durable terminal failure reason.
        reason: String,
    },
}

impl HarnessEvent {
    /// Returns the owning run id without validating event order or phase.
    pub fn run_id(&self) -> &RunId {
        match self {
            Self::RunStarted { run_id, .. }
            | Self::ContextBuilt { run_id, .. }
            | Self::ModelRequested { run_id, .. }
            | Self::ModelResponded { run_id, .. }
            | Self::ToolCallProposed { run_id, .. }
            | Self::PolicyEvaluated { run_id, .. }
            | Self::ApprovalGranted { run_id, .. }
            | Self::ApprovalDenied { run_id, .. }
            | Self::ToolStarted { run_id, .. }
            | Self::ToolFinished { run_id, .. }
            | Self::ToolFailed { run_id, .. }
            | Self::RunFinished { run_id }
            | Self::RunFailed { run_id, .. } => run_id,
        }
    }

    /// Returns the stable snake-case event name used in transition diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RunStarted { .. } => "run_started",
            Self::ContextBuilt { .. } => "context_built",
            Self::ModelRequested { .. } => "model_requested",
            Self::ModelResponded { .. } => "model_responded",
            Self::ToolCallProposed { .. } => "tool_call_proposed",
            Self::PolicyEvaluated { .. } => "policy_evaluated",
            Self::ApprovalGranted { .. } => "approval_granted",
            Self::ApprovalDenied { .. } => "approval_denied",
            Self::ToolStarted { .. } => "tool_started",
            Self::ToolFinished { .. } => "tool_finished",
            Self::ToolFailed { .. } => "tool_failed",
            Self::RunFinished { .. } => "run_finished",
            Self::RunFailed { .. } => "run_failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectClass, ToolCallId, ToolName};
    use serde_json::json;

    #[test]
    fn harness_events_round_trip_as_json() {
        let event = HarnessEvent::ToolCallProposed {
            run_id: RunId::new("run_1").unwrap(),
            turn_id: TurnId::new("turn_1").unwrap(),
            call: ToolCall {
                id: ToolCallId::new("call_1").unwrap(),
                tool: ToolName::new("file.read").unwrap(),
                effect: EffectClass::ReadOnly,
                input: json!({ "path": "README.md" }),
            },
        };

        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: HarnessEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn recorded_events_round_trip_as_json() {
        let event = RecordedEvent {
            seq: 0,
            occurred_at_ms: 1_700_000_000_000,
            event: HarnessEvent::RunStarted {
                run_id: RunId::new("run_1").unwrap(),
                agent_id: AgentId::new("agent_1").unwrap(),
            },
        };

        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: RecordedEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, event);
    }
}
