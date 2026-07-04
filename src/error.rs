//! Error types shared across Platonic Core modules.

/// Core error type for Platonic primitives.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    /// Identifier constructor received an empty value.
    #[error("{0} cannot be empty")]
    EmptyIdentifier(&'static str),

    /// A token budget would be exceeded.
    #[error("context budget exceeded: used {used}, budget {budget}")]
    ContextBudgetExceeded { used: u32, budget: u32 },

    /// A recorded event sequence number was not the expected next value.
    #[error("event sequence mismatch: expected {expected}, actual {actual}")]
    SequenceMismatch { expected: u64, actual: u64 },

    /// An event for one run was applied to another run.
    #[error("run id mismatch: expected {expected}, actual {actual}")]
    RunIdMismatch { expected: String, actual: String },

    /// An event was not legal in the current run phase.
    #[error("invalid transition from {phase} on {event}")]
    InvalidTransition {
        phase: &'static str,
        event: &'static str,
    },

    /// A model response did not match the pending model request.
    #[error("model step mismatch: expected {expected}, actual {actual}")]
    StepMismatch { expected: u32, actual: u32 },

    /// A turn-scoped event did not match the pending turn.
    #[error("turn mismatch: expected {expected}, actual {actual}")]
    TurnMismatch { expected: String, actual: String },

    /// A new turn reused the previous concluded turn id.
    #[error("turn id was reused: {turn_id}")]
    TurnReused { turn_id: String },

    /// A tool event did not match the pending tool call.
    #[error("tool call mismatch: expected {expected}, actual {actual}")]
    ToolCallMismatch { expected: String, actual: String },

    /// A validated tool call did not match any pending model proposal.
    #[error("tool call was not proposed by the model")]
    UnproposedToolCall,
}
