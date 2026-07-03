//! Durable harness event ledger types.

use crate::{
    ContextPack, HenadId, ModelName, PolicyDecision, RunId, ToolCall, ToolCallId, ToolResult,
    TurnId,
};
use serde::{Deserialize, Serialize};

/// Durable event log entries. Transcript, metrics, replay, and audit views are derived from this log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum HarnessEvent {
    /// A run began.
    RunStarted { run_id: RunId, henad_id: HenadId },
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
        model: ModelName,
    },
    /// A model proposed a tool call.
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
    /// Tool execution started.
    ToolStarted { run_id: RunId, call_id: ToolCallId },
    /// Tool execution finished.
    ToolFinished { run_id: RunId, result: ToolResult },
    /// A run completed successfully.
    RunFinished { run_id: RunId },
    /// A run failed.
    RunFailed { run_id: RunId, reason: String },
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
}
