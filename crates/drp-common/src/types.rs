//! Cross-cutting domain value types shared by multiple crates.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::id::AssetId;

/// Kind of catalog asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// Relational table.
    Table,
    /// SQL view.
    View,
    /// Materialized view.
    MaterializedView,
    /// File or object-store path.
    File,
    /// Streaming topic / queue.
    Stream,
    /// Arbitrary / unknown asset.
    #[default]
    Other,
}

/// Logical data type used by profilers and validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    /// Boolean.
    Boolean,
    /// Integer.
    Integer,
    /// Floating point / decimal.
    Float,
    /// UTF-8 string.
    String,
    /// Date without time.
    Date,
    /// Timestamp.
    Timestamp,
    /// Binary blob.
    Binary,
    /// Nested / semi-structured.
    Complex,
    /// Unknown or unmapped type.
    Unknown,
}

/// Severity for checks, alerts, and validation outcomes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational only.
    Info,
    /// Warning.
    Warning,
    /// Error — fail the run.
    #[default]
    Error,
    /// Critical.
    Critical,
}

/// Outcome of a validation / check execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// All assertions passed.
    Passed,
    /// Soft assertions failed.
    Warned,
    /// Hard assertions failed.
    Failed,
    /// Check could not be executed.
    Error,
    /// Check was skipped.
    Skipped,
}

/// Coarse health rollup for assets or services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Healthy.
    Healthy,
    /// Degraded but serving.
    Degraded,
    /// Unhealthy.
    Unhealthy,
    /// No signal yet.
    #[default]
    Unknown,
}

/// Lightweight pointer to an asset in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    /// Asset identifier.
    pub id: AssetId,
    /// Fully-qualified name.
    pub fqn: String,
    /// Asset kind.
    pub kind: AssetKind,
}

impl AssetRef {
    /// Construct a new asset reference.
    pub fn new(id: AssetId, fqn: impl Into<String>, kind: AssetKind) -> Self {
        Self {
            id,
            fqn: fqn.into(),
            kind,
        }
    }
}

/// Physical or logical location of a data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Connector type id.
    pub connector: String,
    /// Opaque location URI or path.
    pub uri: String,
    /// Free-form properties.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

impl SourceLocation {
    /// Construct a location.
    pub fn new(connector: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            connector: connector.into(),
            uri: uri.into(),
            properties: IndexMap::new(),
        }
    }

    /// Attach a property.
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_location_builder() {
        let loc = SourceLocation::new("pg", "postgres://x").with_property("schema", "public");
        assert_eq!(loc.connector, "pg");
        assert_eq!(
            loc.properties.get("schema").map(String::as_str),
            Some("public")
        );
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }
}
