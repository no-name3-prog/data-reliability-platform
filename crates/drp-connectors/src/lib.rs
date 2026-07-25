//! Data source connectors for the Data Reliability Platform.
//!
//! # Built-in connectors
//!
//! | Id | Module | Source |
//! |----|--------|--------|
//! | `postgres` | [`postgres`] | PostgreSQL catalogs + SQL samples |
//! | `csv` | [`csv_file`] | Local CSV files / directories |
//! | `parquet` | [`parquet_file`] | Local Parquet files / directories |
//! | `mock` / `fixture` / `failing` | [`mock`] | Tests and demos |
//!
//! # Adding a database connector
//!
//! 1. Implement [`drp_core::ConnectorPlugin`] (`test_connection`, `discover`,
//!    `discover_catalog`, `sample_rows`).
//! 2. Export `register` or call [`register_builtin_connectors`].
//! 3. Register at the composition root — **do not** change feature services.
//!
//! See `docs/connectors.md`.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod csv_file;
pub mod mock;
pub mod parquet_file;
pub mod postgres;
mod registry;

pub use csv_file::CsvConnector;
pub use mock::{
    row, sample_orders_asset, sample_orders_columns, sample_orders_rows, sample_users_asset,
    sample_users_columns, sample_users_rows, FailingConnector, FixtureConnector, FixtureTable,
    MockConnector, SharedFixtureConnector,
};
pub use parquet_file::ParquetConnector;
pub use postgres::PostgresConnector;
pub use registry::register_builtin_connectors;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
