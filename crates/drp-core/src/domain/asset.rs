//! Catalog asset model.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, AssetKind, DataType, HealthStatus, SourceLocation, UtcTimestamp};

/// A catalogued data asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique id.
    pub id: AssetId,
    /// Fully-qualified name.
    pub fqn: String,
    /// Display name.
    pub name: String,
    /// Asset kind.
    pub kind: AssetKind,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Physical location.
    pub location: SourceLocation,
    /// Column-level metadata when known.
    #[serde(default)]
    pub columns: Vec<ColumnMeta>,
    /// Free-form tags.
    #[serde(default)]
    pub tags: IndexMap<String, String>,
    /// Aggregated health.
    #[serde(default)]
    pub health: HealthStatus,
    /// Creation time.
    pub created_at: UtcTimestamp,
    /// Last update time.
    pub updated_at: UtcTimestamp,
}

impl Asset {
    /// Create a new asset with generated id and timestamps.
    pub fn new(
        fqn: impl Into<String>,
        name: impl Into<String>,
        kind: AssetKind,
        location: SourceLocation,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id: AssetId::new(),
            fqn: fqn.into(),
            name: name.into(),
            kind,
            description: None,
            location,
            columns: Vec::new(),
            tags: IndexMap::new(),
            health: HealthStatus::Unknown,
            created_at: now,
            updated_at: now,
        }
    }

    /// Attach columns.
    pub fn with_columns(mut self, columns: Vec<ColumnMeta>) -> Self {
        self.columns = columns;
        self
    }

    /// Attach a tag.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Column-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    /// Column name.
    pub name: String,
    /// Logical type.
    pub data_type: DataType,
    /// Whether nulls are allowed.
    #[serde(default)]
    pub nullable: bool,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Position in the schema (0-based).
    #[serde(default)]
    pub ordinal: u32,
}

impl ColumnMeta {
    /// Construct a column.
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            description: None,
            ordinal: 0,
        }
    }

    /// Mark non-nullable.
    pub fn required(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Set ordinal position.
    pub fn at(mut self, ordinal: u32) -> Self {
        self.ordinal = ordinal;
        self
    }
}
