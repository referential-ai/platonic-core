//! Context assembly primitives with lane-level budget accounting.

use crate::PlatonicError;
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
