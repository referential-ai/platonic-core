//! Platonic Core: typed harness primitives for disciplined agent execution.
//!
//! The crate intentionally starts as a small kernel, not an agent app. It models
//! runs, context packs, tool calls, policy decisions, and event-log entries. The
//! default stance is: every side effect is typed, policy-gated, and recorded.

#![forbid(unsafe_code)]

pub mod context;
pub mod error;
pub mod event;
pub mod ids;
pub mod message;
pub mod policy;
pub mod tool;

pub use context::{ContextFragment, ContextLane, ContextPack};
pub use error::PlatonicError;
pub use event::HarnessEvent;
pub use ids::{ArtifactId, HenadId, ModelName, RunId, ToolCallId, ToolName, TurnId};
pub use message::{Message, MessageRole};
pub use policy::{EffectClass, PolicyDecision};
pub use tool::{ResultVisibility, ToolCall, ToolResult};
