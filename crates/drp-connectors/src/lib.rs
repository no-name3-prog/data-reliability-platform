//! Connector plugins. Built and tested only inside Docker.
//!
//! Built-in connectors:
//! - [`MockConnector`] — fixed sample data
//! - [`FixtureConnector`] — configurable tables for tests
//! - [`FailingConnector`] — negative-path tests

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod mock;
mod registry;

pub use mock::{
    row, sample_orders_asset, sample_orders_columns, sample_orders_rows, sample_users_asset,
    sample_users_columns, sample_users_rows, FailingConnector, FixtureConnector, FixtureTable,
    MockConnector, SharedFixtureConnector,
};
pub use registry::register_builtin_connectors;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
