//! Storage trait surface.

use async_trait::async_trait;

use drp_common::{AssetId, CheckId, JobId, Result, RunId};
use drp_core::{Asset, CheckDefinition, CheckResult, DatasetProfile, JobDefinition, JobRun};

/// Persistence interface used by all feature crates.
#[async_trait]
pub trait Store: Send + Sync {
    /// Insert or replace an asset.
    async fn upsert_asset(&self, asset: Asset) -> Result<Asset>;
    /// Fetch an asset by id.
    async fn get_asset(&self, id: &AssetId) -> Result<Option<Asset>>;
    /// Fetch an asset by fully-qualified name.
    async fn get_asset_by_fqn(&self, fqn: &str) -> Result<Option<Asset>>;
    /// List assets.
    async fn list_assets(&self, limit: Option<usize>) -> Result<Vec<Asset>>;
    /// Delete an asset.
    async fn delete_asset(&self, id: &AssetId) -> Result<bool>;
    /// Upsert a check definition.
    async fn upsert_check(&self, check: CheckDefinition) -> Result<CheckDefinition>;
    /// Get a check by id.
    async fn get_check(&self, id: &CheckId) -> Result<Option<CheckDefinition>>;
    /// List checks, optionally filtered by asset.
    async fn list_checks(&self, asset_id: Option<&AssetId>) -> Result<Vec<CheckDefinition>>;
    /// Persist a check result.
    async fn save_check_result(&self, result: CheckResult) -> Result<CheckResult>;
    /// List recent check results for a check.
    async fn list_check_results(
        &self,
        check_id: &CheckId,
        limit: Option<usize>,
    ) -> Result<Vec<CheckResult>>;
    /// Append a dataset profile to history (does not overwrite previous runs).
    async fn save_profile(&self, profile: DatasetProfile) -> Result<DatasetProfile>;
    /// Latest profile for an asset.
    async fn latest_profile(&self, asset_id: &AssetId) -> Result<Option<DatasetProfile>>;
    /// Profile history for an asset, newest first.
    async fn list_profile_history(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<DatasetProfile>>;
    /// Fetch a specific profile run by id.
    async fn get_profile_by_run(
        &self,
        asset_id: &AssetId,
        run_id: &RunId,
    ) -> Result<Option<DatasetProfile>>;
    /// Upsert a job definition.
    async fn upsert_job(&self, job: JobDefinition) -> Result<JobDefinition>;
    /// Get a job by id.
    async fn get_job(&self, id: &JobId) -> Result<Option<JobDefinition>>;
    /// List jobs.
    async fn list_jobs(&self) -> Result<Vec<JobDefinition>>;
    /// Save a job run.
    async fn save_job_run(&self, run: JobRun) -> Result<JobRun>;
    /// Get a job run.
    async fn get_job_run(&self, id: &RunId) -> Result<Option<JobRun>>;
    /// List runs for a job.
    async fn list_job_runs(&self, job_id: &JobId, limit: Option<usize>) -> Result<Vec<JobRun>>;
}
