//! Built-in validator registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::{NotNullValidator, RegexValidator, UniqueValidator};

/// Register built-in validators.
pub fn register_builtin_validators(registry: &PluginRegistry) {
    registry.register_validator(Arc::new(NotNullValidator::new()));
    registry.register_validator(Arc::new(UniqueValidator::new()));
    registry.register_validator(Arc::new(RegexValidator::new()));
}
