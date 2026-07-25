//! Validation service: check CRUD, suite execution, history, scheduling helpers.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{json, Value};
use tracing::{info, instrument, warn};

use drp_common::{AssetId, CheckId, Error, JobId, Result, RunId, UtcTimestamp, ValidationStatus};
use drp_core::{
    CheckDefinition, CheckResult, EventBus, JobDefinition, PlatformEvent, PluginContext,
    PluginInfo, PluginRegistry, ValidationRun,
};
use drp_storage::Store;

use crate::engine::RuleEngine;

/// Orchestrates check CRUD, rule execution, suite history, and schedule helpers.
#[derive(Clone)]
pub struct ValidationService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    engine: RuleEngine,
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
        let engine = RuleEngine::new(plugins.clone());
        Self {
            store,
            plugins,
            engine,
            events,
            sample_size,
            fail_fast,
        }
    }

    /// Access the rule engine (list rules, resolve by id).
    pub fn engine(&self) -> &RuleEngine {
        &self.engine
    }

    /// List registered validation rules (validator plugins).
    pub fn list_rules(&self) -> Vec<PluginInfo> {
        self.engine.list_rules()
    }

    /// Create or update a check definition.
    pub async fn upsert_check(&self, check: CheckDefinition) -> Result<CheckDefinition> {
        // Validate that the rule exists early.
        let _ = self.engine.get(&check.validator)?;
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

    /// List result history for a check (newest first).
    pub async fn list_check_results(
        &self,
        check_id: &CheckId,
        limit: Option<usize>,
    ) -> Result<Vec<CheckResult>> {
        // Ensure check exists.
        let _ = self.get_check(check_id).await?;
        self.store.list_check_results(check_id, limit).await
    }

    /// Get a suite run.
    pub async fn get_validation_run(&self, id: &RunId) -> Result<ValidationRun> {
        self.store
            .get_validation_run(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("validation run {id}")))
    }

    /// List suite runs (newest first).
    pub async fn list_validation_runs(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<ValidationRun>> {
        self.store.list_validation_runs(asset_id, limit).await
    }

    /// Run a single check using the given connector for sampling.
    ///
    /// Every execution is **appended** to check-result history.
    #[instrument(skip(self), fields(check_id = %check_id))]
    pub async fn run_check(&self, check_id: &CheckId, connector_id: &str) -> Result<CheckResult> {
        let check = self.get_check(check_id).await?;
        if !check.enabled {
            let skipped = CheckResult {
                run_id: RunId::new(),
                suite_run_id: None,
                check_id: check.id,
                status: ValidationStatus::Skipped,
                severity: check.severity,
                message: "check is disabled".into(),
                metrics: IndexMap::new(),
                finished_at: UtcTimestamp::now(),
            };
            return self.store.save_check_result(skipped).await;
        }

        let result = self.execute_check(&check, connector_id, None, None).await?;
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

    /// Run all enabled checks for an asset and persist a suite run.
    pub async fn run_checks_for_asset(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
    ) -> Result<ValidationRun> {
        self.run_suite(Some(*asset_id), connector_id, None, None)
            .await
    }

    /// Run a validation suite and save history for the suite + each check.
    ///
    /// * `asset_id` — when set, run all enabled checks for that asset (unless
    ///   `check_ids` is provided).
    /// * `check_ids` — optional explicit list of checks.
    /// * `job_id` — optional scheduler job that triggered this suite.
    #[instrument(skip(self, check_ids), fields(connector = %connector_id))]
    pub async fn run_suite(
        &self,
        asset_id: Option<AssetId>,
        connector_id: &str,
        check_ids: Option<Vec<CheckId>>,
        job_id: Option<JobId>,
    ) -> Result<ValidationRun> {
        let started_at = UtcTimestamp::now();
        let checks = self.resolve_checks(asset_id.as_ref(), check_ids).await?;

        // Pre-allocate suite id so individual results can link to it.
        let suite_id = RunId::new();
        let mut results = Vec::with_capacity(checks.len());

        for check in checks {
            if !check.enabled {
                let skipped = CheckResult {
                    run_id: RunId::new(),
                    suite_run_id: Some(suite_id),
                    check_id: check.id,
                    status: ValidationStatus::Skipped,
                    severity: check.severity,
                    message: "check is disabled".into(),
                    metrics: IndexMap::new(),
                    finished_at: UtcTimestamp::now(),
                };
                let saved = self.store.save_check_result(skipped).await?;
                results.push(saved);
                continue;
            }

            let mut result = self
                .execute_check(&check, connector_id, Some(suite_id), None)
                .await?;
            result.suite_run_id = Some(suite_id);
            let saved = self.store.save_check_result(result).await?;

            self.events
                .publish(PlatformEvent::CheckCompleted {
                    check_id: saved.check_id,
                    run_id: saved.run_id,
                    status: saved.status,
                })
                .await;

            let hard_fail = matches!(saved.status, ValidationStatus::Failed)
                && matches!(
                    saved.severity,
                    drp_common::Severity::Error | drp_common::Severity::Critical
                );
            results.push(saved);
            if self.fail_fast && hard_fail {
                break;
            }
        }

        let mut run =
            ValidationRun::from_results(asset_id, connector_id, job_id, started_at, results);
        // Preserve pre-allocated suite id so linked CheckResults match.
        run.id = suite_id;
        let saved = self.store.save_validation_run(run).await?;

        info!(
            run_id = %saved.id,
            status = ?saved.status,
            passed = saved.passed,
            failed = saved.failed,
            "validation suite completed"
        );

        self.events
            .publish(PlatformEvent::ValidationRunCompleted {
                run_id: saved.id,
                asset_id: saved.asset_id,
                status: format!("{:?}", saved.status).to_lowercase(),
            })
            .await;

        Ok(saved)
    }

    /// Create (or update) a scheduled job that runs validation for an asset.
    ///
    /// Job kind is always `validation`. Params: `asset_id`, `connector_id`,
    /// optional `check_ids`.
    pub async fn schedule_asset_validation(
        &self,
        name: impl Into<String>,
        asset_id: AssetId,
        connector_id: impl Into<String>,
        schedule: impl Into<String>,
        check_ids: Option<Vec<CheckId>>,
    ) -> Result<JobDefinition> {
        let connector_id = connector_id.into();
        let mut job = JobDefinition::new(name, "validation").with_schedule(schedule);
        job.params
            .insert("asset_id".into(), json!(asset_id.to_string()));
        job.params
            .insert("connector_id".into(), json!(connector_id));
        if let Some(ids) = check_ids {
            let arr: Vec<Value> = ids.iter().map(|id| json!(id.to_string())).collect();
            job.params.insert("check_ids".into(), Value::Array(arr));
        }
        self.store.upsert_job(job).await
    }

    /// Upsert a check and, if it has a schedule, create a linked validation job.
    pub async fn upsert_check_with_schedule(
        &self,
        mut check: CheckDefinition,
        connector_id: &str,
    ) -> Result<CheckDefinition> {
        let _ = self.engine.get(&check.validator)?;
        if let Some(ref schedule) = check.schedule.clone() {
            let job = self
                .schedule_asset_validation(
                    format!("check:{}", check.name),
                    check.asset_id,
                    connector_id,
                    schedule,
                    Some(vec![check.id]),
                )
                .await?;
            check.job_id = Some(job.id);
        }
        self.store.upsert_check(check).await
    }

    async fn resolve_checks(
        &self,
        asset_id: Option<&AssetId>,
        check_ids: Option<Vec<CheckId>>,
    ) -> Result<Vec<CheckDefinition>> {
        if let Some(ids) = check_ids {
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                out.push(self.get_check(&id).await?);
            }
            return Ok(out);
        }
        let asset_id =
            asset_id.ok_or_else(|| Error::validation("asset_id or check_ids is required"))?;
        self.list_checks(Some(asset_id)).await
    }

    async fn execute_check(
        &self,
        check: &CheckDefinition,
        connector_id: &str,
        suite_run_id: Option<RunId>,
        extra_ctx: Option<IndexMap<String, Value>>,
    ) -> Result<CheckResult> {
        let asset = self
            .store
            .get_asset(&check.asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {}", check.asset_id)))?;

        let connector = self.plugins.connector(connector_id)?;
        let mut ctx = PluginContext::new();
        if let Some(extra) = extra_ctx {
            ctx.config = extra;
        }

        // Resolve referential integrity reference asset into context.
        if check.validator == "referential_integrity" {
            if let Err(e) = self
                .inject_reference_values(check, connector_id, &mut ctx)
                .await
            {
                warn!(error = %e, "failed to load reference values");
                let mut r = CheckResult::error(check.id, check.severity, e.to_string());
                if let Some(sid) = suite_run_id {
                    r = r.with_suite_run(sid);
                }
                return Ok(r);
            }
        }

        let rows = match connector
            .sample_rows(&asset, self.sample_size, &PluginContext::new())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let mut r =
                    CheckResult::error(check.id, check.severity, format!("sample failed: {e}"));
                if let Some(sid) = suite_run_id {
                    r = r.with_suite_run(sid);
                }
                return Ok(r);
            }
        };

        match self.engine.execute(check, &asset, &rows, &ctx).await {
            Ok(mut result) => {
                if let Some(sid) = suite_run_id {
                    result = result.with_suite_run(sid);
                }
                Ok(result)
            }
            Err(e) => {
                let mut r = CheckResult::error(check.id, check.severity, e.to_string());
                if let Some(sid) = suite_run_id {
                    r = r.with_suite_run(sid);
                }
                Ok(r)
            }
        }
    }

    async fn inject_reference_values(
        &self,
        check: &CheckDefinition,
        connector_id: &str,
        ctx: &mut PluginContext,
    ) -> Result<()> {
        // Prefer explicit values on the check.
        if check
            .params
            .get("values")
            .and_then(|v| v.as_array())
            .is_some()
        {
            return Ok(());
        }

        let ref_asset_id = check
            .params
            .get("reference_asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::validation(
                    "referential_integrity needs 'values' or 'reference_asset_id' + 'reference_column'",
                )
            })?;
        let ref_col = check
            .params
            .get("reference_column")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::validation("referential_integrity requires 'reference_column'")
            })?;

        let ref_id: AssetId = ref_asset_id.parse()?;
        let ref_asset = self
            .store
            .get_asset(&ref_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("reference asset {ref_id}")))?;

        // Allow optional override of connector for the reference asset.
        let ref_connector_id = check
            .params
            .get("reference_connector")
            .and_then(|v| v.as_str())
            .unwrap_or(connector_id);
        let connector = self.plugins.connector(ref_connector_id)?;
        let rows = connector
            .sample_rows(&ref_asset, self.sample_size, &PluginContext::new())
            .await?;

        let mut values = Vec::new();
        for row in rows {
            if let Some(v) = row.get(ref_col) {
                if !v.is_null() {
                    values.push(v.clone());
                }
            }
        }
        ctx.config
            .insert("reference_values".into(), Value::Array(values));
        Ok(())
    }
}
