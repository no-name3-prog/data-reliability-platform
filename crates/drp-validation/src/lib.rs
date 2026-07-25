//! Data quality validation engine.
//!
//! Built-in rules: not_null, unique, accepted_values, regex, range, freshness,
//! row_count, referential_integrity.
//!
//! New rules implement [`drp_core::ValidatorPlugin`] and register on the
//! [`drp_core::PluginRegistry`]. See [`engine::RuleEngine`] and `docs/validation.md`.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod builtins;
mod engine;
mod job;
mod registry;
mod service;

/// Shared parameter helpers for writing new validator plugins.
pub mod params;

pub use builtins::{
    AcceptedValuesValidator, FreshnessValidator, NotNullValidator, RangeValidator,
    ReferentialIntegrityValidator, RegexValidator, RowCountValidator, UniqueValidator,
};
pub use engine::RuleEngine;
pub use job::{ValidationJobHandler, VALIDATION_JOB_KIND};
pub use registry::register_builtin_validators;
pub use service::ValidationService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
