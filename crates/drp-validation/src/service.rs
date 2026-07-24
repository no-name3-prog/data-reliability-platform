//! Validation service.

use std::sync::Arc;

use tracing::instrument;

use drp_common::{AssetId, CheckId, Error, Result, ValidationStatus};
use drp_core::{
    CheckDefinition, CheckResult, EventBus, PlatformEvent, PluginContext, PluginRegistry,
};
use drp_storage::Store;

/// Orchestrates check CRUD and execution.
#[derive(Clone)]
pub struct ValidationService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    events: EventBus,
    sample_size: usize,
    fail_fast: bool,
}

impl ValidationService {
    /// Create a validation service.
    pub fn new(
        store: Arc<dyn Store>,
        plugins: PluginRegistry,
        events: EventBus,
        sample_size: usize,
        fail_fast: bool,
    ) -> Self {
        Self {
            store,
            plugins,
            events,
            sample_size,
            fail_fast,
        }
    }

    /// Create or update a check definition.
    pub async fn upsert_check(&self, check: CheckDefinition) -> Result<CheckDefinition> {
        self.store.upsert_check(check).await
    }

    /// Get a check.
    pub async fn get_check(&self, id: &CheckId) -> Result<CheckDefinition> {
        self.store
            .get_check(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("check {id}")))
    }

    /// List checks.
    pub async fn list_checks(&self, asset_id: Option<&AssetId>) -> Result<Vec<CheckDefinition>> {
        self.store.list_checks(asset_id).await
    }

    /// Run a single check using the given connector for sampling.
    #[instrument(skip(self), fields(check_id = %check_id))]
    pub async fn run_check(&self, check_id: &CheckId, connector_id: &str) -> Result<CheckResult> {
        let check = self.get_check(check_id).await?;
        if !check.enabled {
            return Ok(CheckResult {
                run_id: drp_common::RunId::new(),
                check_id: check.id,
                status: ValidationStatus::Skipped,
                severity: check.severity,
                message: "check is disabled".into(),
                metrics: indexmap::IndexMap::new(),
                finished_at: drp_common::UtcTimestamp::now(),
            });
        }

        let asset = self
            .store
            .get_asset(&check.asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {}", check.asset_id)))?;

        let connector = self.plugins.connector(connector_id)?;
        let validator = self.plugins.validator(&check.validator)?;
        let ctx = PluginContext::new();
        let rows = connector
            .sample_rows(&asset, self.sample_size, &ctx)
            .await?;
        let result = validator.validate(&check, &asset, &rows, &ctx).await?;
        let saved = self.store.save_check_result(result).await?;

        self.events
            .publish(PlatformEvent::CheckCompleted {
                check_id: saved.check_id,
                run_id: saved.run_id,
                status: saved.status,
            })
            .await;

        Ok(saved)
    }

    /// Run all enabled checks for an asset.
    pub async fn run_checks_for_asset(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
    ) -> Result<Vec<CheckResult>> {
        let checks = self.list_checks(Some(asset_id)).await?;
        let mut results = Vec::new();
        for check in checks {
            if !check.enabled {
                continue;
            }
            let result = self.run_check(&check.id, connector_id).await?;
            let hard_fail = matches!(result.status, ValidationStatus::Failed)
                && matches!(
                    result.severity,
                    drp_common::Severity::Error | drp_common::Severity::Critical
                );
            results.push(result);
            if self.fail_fast && hard_fail {
                break;
            }
        }
        Ok(results)
    }
}
