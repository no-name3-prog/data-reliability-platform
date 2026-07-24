//! Shared test harnesses for unit, integration, and regression suites.
//!
//! This crate is **not published** and is intended only for tests that run
//! inside Docker via `cargo nextest` / `make test`.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod fixtures;
pub mod harness;
pub mod http;

pub use fixtures::{load_json_fixture, orders_null_email_check, regression_orders_fixture};
pub use harness::{PlatformHarness, TestPlatform};
pub use http::{get_json, post_json, response_json, TestClient};
