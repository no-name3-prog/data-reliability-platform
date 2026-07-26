//! Storage trait surface.

use async_trait::async_trait;

use drp_common::{AssetId, CheckId, IncidentId, JobId, Result, RunId, SuggestionId};
use drp_core::{
    AnomalyReport, Asset, CheckDefinition, CheckResult, DatasetProfile, Incident,
    IncidentTimelineEvent, JobDefinition, JobRun, RuleSuggestion, RuleSuggestionStatus,
    ValidationRun,
};

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
    /// Persist a check result (appends history; never overwrites).
    async fn save_check_result(&self, result: CheckResult) -> Result<CheckResult>;
    /// List recent check results for a check (newest first).
    async fn list_check_results(
        &self,
        check_id: &CheckId,
        limit: Option<usize>,
    ) -> Result<Vec<CheckResult>>;
    /// Persist a validation suite run (appends history).
    async fn save_validation_run(&self, run: ValidationRun) -> Result<ValidationRun>;
    /// Fetch a suite run by id.
    async fn get_validation_run(&self, id: &RunId) -> Result<Option<ValidationRun>>;
    /// List suite runs, optionally filtered by asset (newest first).
    async fn list_validation_runs(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<ValidationRun>>;
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

    /// Append an anomaly report.
    async fn save_anomaly_report(&self, report: AnomalyReport) -> Result<AnomalyReport>;
    /// Get an anomaly report by run id.
    async fn get_anomaly_report(&self, run_id: &RunId) -> Result<Option<AnomalyReport>>;
    /// List anomaly reports for an asset (newest first).
    async fn list_anomaly_reports(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<AnomalyReport>>;
    /// Persist an incident.
    async fn save_incident(&self, incident: Incident) -> Result<Incident>;
    /// Get an incident by id.
    async fn get_incident(&self, id: &IncidentId) -> Result<Option<Incident>>;
    /// List incidents, optionally filtered by asset (newest first).
    async fn list_incidents(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<Incident>>;
    /// Append a timeline / history event for an incident.
    async fn append_incident_event(
        &self,
        event: IncidentTimelineEvent,
    ) -> Result<IncidentTimelineEvent>;
    /// List timeline events for an incident (oldest first).
    async fn list_incident_events(
        &self,
        incident_id: &IncidentId,
        limit: Option<usize>,
    ) -> Result<Vec<IncidentTimelineEvent>>;
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

    /// Upsert an AI rule suggestion.
    async fn upsert_rule_suggestion(&self, suggestion: RuleSuggestion) -> Result<RuleSuggestion>;
    /// Get a rule suggestion by id.
    async fn get_rule_suggestion(&self, id: &SuggestionId) -> Result<Option<RuleSuggestion>>;
    /// List rule suggestions (newest first), optionally filtered by asset and status.
    async fn list_rule_suggestions(
        &self,
        asset_id: Option<&AssetId>,
        status: Option<RuleSuggestionStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<RuleSuggestion>>;
}
