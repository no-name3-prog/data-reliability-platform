//! Typed identifiers.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Ulid);

        impl $name {
            /// Generate a new ULID-backed identifier.
            pub fn new() -> Self {
                Self(Ulid::new())
            }
            /// Construct from a raw ULID.
            pub fn from_ulid(id: Ulid) -> Self {
                Self(id)
            }
            /// Borrow the inner ULID.
            pub fn as_ulid(&self) -> Ulid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = crate::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ulid::from_string(s)
                    .map(Self)
                    .map_err(|e| crate::Error::validation(format!("invalid {}: {e}", stringify!($name))))
            }
        }

        impl From<Ulid> for $name {
            fn from(id: Ulid) -> Self {
                Self(id)
            }
        }
    };
}

typed_id!(/// Identifier for a catalog asset.
    AssetId);
typed_id!(/// Identifier for a logical dataset.
    DatasetId);
typed_id!(/// Identifier for a data-quality check.
    CheckId);
typed_id!(/// Identifier for a job.
    JobId);
typed_id!(/// Identifier for a single execution run.
    RunId);
typed_id!(/// Identifier for a tenant / workspace.
    TenantId);
typed_id!(/// Identifier for a registered plugin instance.
    PluginId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_display_parse() {
        let id = AssetId::new();
        let parsed: AssetId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }
}
