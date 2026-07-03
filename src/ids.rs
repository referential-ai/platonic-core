//! Compact identifier newtypes used across the harness event ledger.

use crate::PlatonicError;
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_empty_values() {
        assert!(matches!(
            RunId::new("  "),
            Err(PlatonicError::EmptyIdentifier("RunId"))
        ));
    }

    #[test]
    fn identifiers_display_their_inner_value() {
        let id = HenadId::new("henad_alpha").unwrap();
        assert_eq!(id.to_string(), "henad_alpha");
        assert_eq!(id.as_str(), "henad_alpha");
    }
}
