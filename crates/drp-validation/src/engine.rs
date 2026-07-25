//! Rule engine facade over the plugin registry.
//!
//! # Adding a new validation rule
//!
//! 1. Implement [`drp_core::ValidatorPlugin`] (and [`drp_core::Plugin`]).
//! 2. Give it a stable `PluginInfo::id` (e.g. `"my_rule"`).
//! 3. Register it in [`crate::register_builtin_validators`] **or** from a
//!    separate plugin crate at the API composition root.
//! 4. Create [`drp_core::CheckDefinition`] rows with `validator = "my_rule"`
//!    and rule-specific `params`.
//!
//! No core trait changes are required for new rules.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;
use tracing::instrument;

use drp_common::Result;
use drp_core::{
    Asset, CheckDefinition, CheckResult, PluginContext, PluginInfo, PluginRegistry, ValidatorPlugin,
};

/// Thin facade that resolves validators by id and runs them.
///
/// Prefer this over calling `PluginRegistry` directly from orchestration code
/// so rule lookup stays consistent.
#[derive(Clone)]
pub struct RuleEngine {
    plugins: PluginRegistry,
}

impl RuleEngine {
    /// Wrap a plugin registry.
    pub fn new(plugins: PluginRegistry) -> Self {
        Self { plugins }
    }

    /// List registered validator plugins.
    pub fn list_rules(&self) -> Vec<PluginInfo> {
        self.plugins
            .list_all()
            .into_iter()
            .filter(|p| {
                p.capabilities
                    .iter()
                    .any(|c| matches!(c, drp_core::PluginCapability::Validator))
            })
            .collect()
    }

    /// Resolve a validator by plugin id.
    pub fn get(&self, validator_id: &str) -> Result<Arc<dyn ValidatorPlugin>> {
        self.plugins.validator(validator_id)
    }

    /// Execute one check definition against rows.
    #[instrument(skip(self, asset, rows, ctx), fields(validator = %check.validator, check_id = %check.id))]
    pub async fn execute(
        &self,
        check: &CheckDefinition,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<CheckResult> {
        let validator = self.get(&check.validator)?;
        validator.validate(check, asset, rows, ctx).await
    }
}
