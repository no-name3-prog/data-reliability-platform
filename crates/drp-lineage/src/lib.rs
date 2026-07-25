//! Lineage graph engine: SQL parsing, table/column lineage, impact analysis.
//!
//! Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod graph;
mod service;
mod sql_parse;

pub use graph::{LineageGraph, LineageSnapshot};
pub use service::{LineageService, SqlIngestResult};
pub use sql_parse::{extract_lineage_from_sql, ColumnMapping, SqlLineageExtract};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
