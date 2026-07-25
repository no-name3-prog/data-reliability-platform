//! Built-in validator registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::{
    AcceptedValuesValidator, FreshnessValidator, NotNullValidator, RangeValidator,
    ReferentialIntegrityValidator, RegexValidator, RowCountValidator, UniqueValidator,
};

/// Register all built-in validation rules on the plugin registry.
///
/// To add a new built-in rule: implement `ValidatorPlugin`, then register here.
/// For external rules, call `registry.register_validator` from the API composition root.
pub fn register_builtin_validators(registry: &PluginRegistry) {
    registry.register_validator(Arc::new(NotNullValidator::new()));
    registry.register_validator(Arc::new(UniqueValidator::new()));
    registry.register_validator(Arc::new(AcceptedValuesValidator::new()));
    registry.register_validator(Arc::new(RegexValidator::new()));
    registry.register_validator(Arc::new(RangeValidator::new()));
    registry.register_validator(Arc::new(FreshnessValidator::new()));
    registry.register_validator(Arc::new(RowCountValidator::new()));
    registry.register_validator(Arc::new(ReferentialIntegrityValidator::new()));
}
