//! Platonic Core: typed harness primitives for disciplined agent execution.
//!
//! The crate intentionally starts as a small kernel, not an agent app. It models
//! runs, context packs, tool calls, policy decisions, and event-log entries. The
//! default stance is: every side effect is typed, policy-gated, and recorded.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Defines a compact string-backed identifier newtype.
macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Creates a new identifier from a non-empty string.
            pub fn new(value: impl Into<String>) -> Result<Self, PlatonicError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(PlatonicError::EmptyIdentifier(stringify!($name)));
                }
                Ok(Self(value))
            }

            /// Returns the identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(RunId, "Identifier for one durable harness run.");
id_type!(TurnId, "Identifier for one model/tool turn inside a run.");
id_type!(HenadId, "Identifier for one bounded agent unit.");
id_type!(ToolCallId, "Identifier for one proposed tool invocation.");
id_type!(
    ArtifactId,
    "Identifier for a durable artifact emitted by a run."
);
id_type!(ToolName, "Stable registered tool name.");
id_type!(ModelName, "Stable model identifier as selected by policy.");

/// Core error type for Platonic primitives.
#[derive(Debug, thiserror::Error)]
pub enum PlatonicError {
    /// Identifier constructor received an empty value.
    #[error("{0} cannot be empty")]
    EmptyIdentifier(&'static str),

    /// A token budget would be exceeded.
    #[error("context budget exceeded: used {used}, budget {budget}")]
    ContextBudgetExceeded { used: u32, budget: u32 },
}

/// Role for a message visible to a model adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// Stable operating contract and runtime rules.
    System,
    /// Human or upstream caller input.
    User,
    /// Model-authored message.
    Assistant,
    /// Tool result observation.
    Tool,
}

/// Message passed through the model boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Message role.
    pub role: MessageRole,
    /// Plain text content. Richer content blocks belong above this core layer.
    pub content: String,
}

/// Lane accounting for context assembly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextLane {
    /// Stable system contract.
    SystemContract,
    /// Current user task.
    CurrentTask,
    /// Selected tool schemas only.
    ToolSchemas,
    /// Recent turns preserved verbatim.
    RecentTurns,
    /// Retrieved memories or project facts.
    RetrievedContext,
    /// Artifact summaries instead of raw large blobs.
    ArtifactSummary,
    /// Runtime policy and approval constraints.
    Policy,
}

/// One accountable context fragment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextFragment {
    /// Context lane this fragment belongs to.
    pub lane: ContextLane,
    /// Human-readable source path, URL, event id, or synthetic label.
    pub source: String,
    /// Text injected into the model prompt.
    pub content: String,
    /// Estimated token count used for budget checks.
    pub estimated_tokens: u32,
}

/// A bounded prompt/context bundle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextPack {
    /// Maximum allowed prompt tokens for this pack.
    pub token_budget: u32,
    /// Context fragments selected for the next model call.
    pub fragments: Vec<ContextFragment>,
}

impl ContextPack {
    /// Returns estimated token usage across fragments.
    pub fn estimated_tokens(&self) -> u32 {
        self.fragments
            .iter()
            .map(|fragment| fragment.estimated_tokens)
            .sum()
    }

    /// Verifies the context pack fits inside its budget.
    pub fn validate_budget(&self) -> Result<(), PlatonicError> {
        let used = self.estimated_tokens();
        if used > self.token_budget {
            return Err(PlatonicError::ContextBudgetExceeded {
                used,
                budget: self.token_budget,
            });
        }
        Ok(())
    }
}

/// High-level class of effect a tool may produce.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    /// Reads local or remote state without mutation.
    ReadOnly,
    /// Mutates files in an explicit workspace.
    WorkspaceWrite,
    /// Performs network IO without an external irreversible side effect.
    Network,
    /// Sends, publishes, charges, deploys, deletes, or otherwise affects the world.
    ExternalSideEffect,
    /// Requests access to credentials, secrets, or protected material.
    SecretAccess,
}

impl EffectClass {
    /// Returns the default policy posture for this effect class.
    pub fn default_policy(&self) -> PolicyDecision {
        match self {
            Self::ReadOnly => PolicyDecision::Allow,
            Self::WorkspaceWrite | Self::Network => PolicyDecision::RequireApproval {
                reason: "mutable or networked tool call requires explicit policy allowance".into(),
            },
            Self::ExternalSideEffect | Self::SecretAccess => PolicyDecision::Deny {
                reason: "external side effects and secret access fail closed by default".into(),
            },
        }
    }
}

/// Policy decision for a proposed model or tool action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum PolicyDecision {
    /// Action may proceed.
    Allow,
    /// Action may proceed only after approval.
    RequireApproval { reason: String },
    /// Action must not proceed.
    Deny { reason: String },
}

/// A tool invocation proposed by a model and evaluated by policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Stable call id assigned by the harness.
    pub id: ToolCallId,
    /// Registered tool name.
    pub tool: ToolName,
    /// Declared effect class.
    pub effect: EffectClass,
    /// JSON input validated against the registered tool schema.
    pub input: Value,
}

/// Controls which audience sees a tool result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultVisibility {
    /// Only model receives it.
    Model,
    /// Only user-facing transcript receives it.
    User,
    /// Model and user-facing transcript both receive it.
    Both,
}

/// Structured tool result; raw output should be kept as an artifact when large.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call this result answers.
    pub call_id: ToolCallId,
    /// Short model-usable summary.
    pub summary: String,
    /// Structured data result.
    pub data: Value,
    /// Artifacts created or referenced by this result.
    pub artifacts: Vec<ArtifactId>,
    /// Visibility boundary for this result.
    pub visibility: ResultVisibility,
}

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
    use serde_json::json;

    #[test]
    fn context_budget_is_enforced() {
        let pack = ContextPack {
            token_budget: 10,
            fragments: vec![ContextFragment {
                lane: ContextLane::CurrentTask,
                source: "user".into(),
                content: "test".into(),
                estimated_tokens: 11,
            }],
        };

        assert!(matches!(
            pack.validate_budget(),
            Err(PlatonicError::ContextBudgetExceeded {
                used: 11,
                budget: 10
            })
        ));
    }

    #[test]
    fn external_side_effects_fail_closed_by_default() {
        assert!(matches!(
            EffectClass::ExternalSideEffect.default_policy(),
            PolicyDecision::Deny { .. }
        ));
    }

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
