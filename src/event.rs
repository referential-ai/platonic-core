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
    /// A run began.
    RunStarted { run_id: RunId, agent_id: AgentId },
    /// Context was assembled for a model call.
    ContextBuilt {
        run_id: RunId,
        turn_id: TurnId,
        context: ContextPack,
    },
    /// A model request crossed the provider boundary.
    ModelRequested {
        run_id: RunId,
        turn_id: TurnId,
        step: u32,
        model: ModelName,
    },
    /// A model response crossed the provider boundary.
    ModelResponded {
        run_id: RunId,
        turn_id: TurnId,
        step: u32,
        output: Message,
        proposed_calls: Vec<ToolProposal>,
        usage: ModelUsage,
    },
    /// A model proposal was accepted as a host-validated tool call.
    ToolCallProposed {
        run_id: RunId,
        turn_id: TurnId,
        call: ToolCall,
    },
    /// Policy evaluated a proposed tool call.
    PolicyEvaluated {
        run_id: RunId,
        call_id: ToolCallId,
        decision: PolicyDecision,
    },
    /// A pending approval was granted.
    ApprovalGranted {
        run_id: RunId,
        call_id: ToolCallId,
        actor_id: ActorId,
    },
    /// A pending approval was denied.
    ApprovalDenied {
        run_id: RunId,
        call_id: ToolCallId,
        actor_id: ActorId,
        reason: String,
    },
    /// Tool execution started.
    ToolStarted { run_id: RunId, call_id: ToolCallId },
    /// Tool execution finished.
    ToolFinished { run_id: RunId, result: ToolResult },
    /// Tool execution failed.
    ToolFailed {
        run_id: RunId,
        call_id: ToolCallId,
        reason: String,
    },
    /// A run completed successfully.
    RunFinished { run_id: RunId },
    /// A run failed.
    RunFailed { run_id: RunId, reason: String },
}

impl HarnessEvent {
    /// Returns the run id associated with this event.
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

    /// Returns a stable event name for diagnostics.
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
