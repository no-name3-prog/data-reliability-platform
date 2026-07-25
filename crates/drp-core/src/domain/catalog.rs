//! Hierarchical catalog model produced by connectors during discovery.
//!
//! ```text
//! CatalogTree
//!   └── Database
//!         └── Schema
//!               └── Table / View / File
//!                     └── Column
//! ```
//!
//! Connectors that only expose flat tables can still populate a single synthetic
//! database/schema (e.g. `default.public`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::domain::asset::ColumnMeta;
use drp_common::{AssetKind, DataType, SourceLocation};

/// Full hierarchical snapshot returned by [`crate::ConnectorPlugin::discover_catalog`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogTree {
    /// Connector plugin id that produced this tree.
    pub connector: String,
    /// Root location that was scanned.
    pub location: SourceLocation,
    /// Databases (or logical namespaces).
    pub databases: Vec<CatalogDatabase>,
}

/// Database / catalog node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogDatabase {
    /// Database name.
    pub name: String,
    /// Schemas in this database.
    pub schemas: Vec<CatalogSchema>,
    /// Free-form properties.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

/// Schema node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSchema {
    /// Schema name.
    pub name: String,
    /// Relations (tables, views, files).
    pub tables: Vec<CatalogTable>,
    /// Free-form properties.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

/// Table / view / file relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogTable {
    /// Relation name.
    pub name: String,
    /// Kind.
    pub kind: AssetKind,
    /// Fully-qualified name (`db.schema.table` or path).
    pub fqn: String,
    /// Columns.
    #[serde(default)]
    pub columns: Vec<ColumnMeta>,
    /// Estimated row count when known.
    #[serde(default)]
    pub row_count_estimate: Option<u64>,
    /// Free-form properties (file path, OID, …).
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

impl CatalogTree {
    /// Create an empty tree for a connector/location.
    pub fn new(connector: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            connector: connector.into(),
            location,
            databases: Vec::new(),
        }
    }

    /// Flatten all tables for asset registration.
    pub fn all_tables(&self) -> Vec<&CatalogTable> {
        self.databases
            .iter()
            .flat_map(|d| d.schemas.iter().flat_map(|s| s.tables.iter()))
            .collect()
    }

    /// Count tables across the tree.
    pub fn table_count(&self) -> usize {
        self.all_tables().len()
    }
}

impl CatalogDatabase {
    /// New empty database.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schemas: Vec::new(),
            properties: IndexMap::new(),
        }
    }
}

impl CatalogSchema {
    /// New empty schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tables: Vec::new(),
            properties: IndexMap::new(),
        }
    }
}

impl CatalogTable {
    /// New table with FQN parts.
    pub fn new(database: &str, schema: &str, name: impl Into<String>, kind: AssetKind) -> Self {
        let name = name.into();
        Self {
            fqn: format!("{database}.{schema}.{name}"),
            name,
            kind,
            columns: Vec::new(),
            row_count_estimate: None,
            properties: IndexMap::new(),
        }
    }

    /// File-style FQN (path as name).
    pub fn file(path: impl Into<String>) -> Self {
        let name = path.into();
        Self {
            fqn: name.clone(),
            name,
            kind: AssetKind::File,
            columns: Vec::new(),
            row_count_estimate: None,
            properties: IndexMap::new(),
        }
    }

    /// Attach columns.
    pub fn with_columns(mut self, columns: Vec<ColumnMeta>) -> Self {
        self.columns = columns;
        self
    }

    /// Attach a property.
    pub fn with_property(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.properties.insert(k.into(), v.into());
        self
    }
}

/// Map SQL / arrow type strings to platform [`DataType`].
pub fn map_sql_type(raw: &str) -> DataType {
    let t = raw.to_ascii_lowercase();
    if t.contains("bool") {
        DataType::Boolean
    } else if t.contains("int") || t.contains("serial") || t == "oid" {
        DataType::Integer
    } else if t.contains("float")
        || t.contains("double")
        || t.contains("numeric")
        || t.contains("decimal")
        || t.contains("real")
        || t.contains("money")
    {
        DataType::Float
    } else if t.contains("timestamp") || t.contains("timestamptz") {
        DataType::Timestamp
    } else if t == "date" {
        DataType::Date
    } else if t.contains("bytea") || t.contains("binary") || t.contains("blob") {
        DataType::Binary
    } else if t.contains("json") || t.contains("array") || t.contains("record") {
        DataType::Complex
    } else if t.contains("char") || t.contains("text") || t.contains("uuid") || t.contains("xml") {
        DataType::String
    } else {
        DataType::Unknown
    }
}
