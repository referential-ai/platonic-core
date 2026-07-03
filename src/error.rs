//! Error types shared across Platonic Core modules.

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
