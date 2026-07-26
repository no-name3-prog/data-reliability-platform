//! In-memory store.

use std::collections::HashMap;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::traits::Store;
use drp_common::{AssetId, CheckId, IncidentId, JobId, Result, RunId, SuggestionId};
use drp_core::{
    AnomalyReport, Asset, CheckDefinition, CheckResult, DatasetProfile, Incident,
    IncidentTimelineEvent, JobDefinition, JobRun, RuleSuggestion, RuleSuggestionStatus,
    ValidationRun,
};

/// Thread-safe in-memory implementation of [`Store`].
#[derive(Debug, Default)]
pub struct MemoryStore {
    assets: RwLock<HashMap<AssetId, Asset>>,
    assets_by_fqn: RwLock<HashMap<String, AssetId>>,
    checks: RwLock<HashMap<CheckId, CheckDefinition>>,
    check_results: RwLock<HashMap<CheckId, Vec<CheckResult>>>,
    validation_runs: RwLock<HashMap<RunId, ValidationRun>>,
    validation_runs_order: RwLock<Vec<RunId>>,
    /// Profile history per asset (oldest first).
    profiles: RwLock<HashMap<AssetId, Vec<DatasetProfile>>>,
    anomaly_reports: RwLock<HashMap<RunId, AnomalyReport>>,
    anomaly_reports_by_asset: RwLock<HashMap<AssetId, Vec<RunId>>>,
    incidents: RwLock<HashMap<IncidentId, Incident>>,
    incidents_order: RwLock<Vec<IncidentId>>,
    incident_events: RwLock<HashMap<IncidentId, Vec<IncidentTimelineEvent>>>,
    jobs: RwLock<HashMap<JobId, JobDefinition>>,
    job_runs: RwLock<HashMap<RunId, JobRun>>,
    job_runs_by_job: RwLock<HashMap<JobId, Vec<RunId>>>,
    rule_suggestions: RwLock<HashMap<SuggestionId, RuleSuggestion>>,
    rule_suggestions_order: RwLock<Vec<SuggestionId>>,
}

impl MemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn upsert_asset(&self, asset: Asset) -> Result<Asset> {
        self.assets_by_fqn
            .write()
            .insert(asset.fqn.clone(), asset.id);
        self.assets.write().insert(asset.id, asset.clone());
        Ok(asset)
    }

    async fn get_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        Ok(self.assets.read().get(id).cloned())
    }

    async fn get_asset_by_fqn(&self, fqn: &str) -> Result<Option<Asset>> {
        let id = self.assets_by_fqn.read().get(fqn).copied();
        Ok(id.and_then(|id| self.assets.read().get(&id).cloned()))
    }

    async fn list_assets(&self, limit: Option<usize>) -> Result<Vec<Asset>> {
        let mut items: Vec<_> = self.assets.read().values().cloned().collect();
        items.sort_by(|a, b| a.fqn.cmp(&b.fqn));
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn delete_asset(&self, id: &AssetId) -> Result<bool> {
        if let Some(asset) = self.assets.write().remove(id) {
            self.assets_by_fqn.write().remove(&asset.fqn);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn upsert_check(&self, check: CheckDefinition) -> Result<CheckDefinition> {
        self.checks.write().insert(check.id, check.clone());
        Ok(check)
    }

    async fn get_check(&self, id: &CheckId) -> Result<Option<CheckDefinition>> {
        Ok(self.checks.read().get(id).cloned())
    }

    async fn list_checks(&self, asset_id: Option<&AssetId>) -> Result<Vec<CheckDefinition>> {
        Ok(self
            .checks
            .read()
            .values()
            .filter(|c| asset_id.map(|id| c.asset_id == *id).unwrap_or(true))
            .cloned()
            .collect())
    }

    async fn save_check_result(&self, result: CheckResult) -> Result<CheckResult> {
        self.check_results
            .write()
            .entry(result.check_id)
            .or_default()
            .push(result.clone());
        Ok(result)
    }

    async fn list_check_results(
        &self,
        check_id: &CheckId,
        limit: Option<usize>,
    ) -> Result<Vec<CheckResult>> {
        let mut items = self
            .check_results
            .read()
            .get(check_id)
            .cloned()
            .unwrap_or_default();
        items.reverse();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn save_validation_run(&self, run: ValidationRun) -> Result<ValidationRun> {
        self.validation_runs_order.write().push(run.id);
        self.validation_runs.write().insert(run.id, run.clone());
        Ok(run)
    }

    async fn get_validation_run(&self, id: &RunId) -> Result<Option<ValidationRun>> {
        Ok(self.validation_runs.read().get(id).cloned())
    }

    async fn list_validation_runs(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<ValidationRun>> {
        let order = self.validation_runs_order.read().clone();
        let map = self.validation_runs.read();
        let mut items: Vec<_> = order
            .into_iter()
            .rev()
            .filter_map(|id| map.get(&id).cloned())
            .filter(|r| {
                asset_id
                    .map(|aid| r.asset_id.as_ref() == Some(aid))
                    .unwrap_or(true)
            })
            .collect();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn save_profile(&self, profile: DatasetProfile) -> Result<DatasetProfile> {
        self.profiles
            .write()
            .entry(profile.asset_id)
            .or_default()
            .push(profile.clone());
        Ok(profile)
    }

    async fn latest_profile(&self, asset_id: &AssetId) -> Result<Option<DatasetProfile>> {
        Ok(self
            .profiles
            .read()
            .get(asset_id)
            .and_then(|v| v.last().cloned()))
    }

    async fn list_profile_history(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<DatasetProfile>> {
        let mut items = self
            .profiles
            .read()
            .get(asset_id)
            .cloned()
            .unwrap_or_default();
        items.reverse(); // newest first
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn get_profile_by_run(
        &self,
        asset_id: &AssetId,
        run_id: &RunId,
    ) -> Result<Option<DatasetProfile>> {
        Ok(self
            .profiles
            .read()
            .get(asset_id)
            .and_then(|v| v.iter().find(|p| p.run_id == *run_id).cloned()))
    }

    async fn save_anomaly_report(&self, report: AnomalyReport) -> Result<AnomalyReport> {
        self.anomaly_reports_by_asset
            .write()
            .entry(report.asset_id)
            .or_default()
            .push(report.run_id);
        self.anomaly_reports
            .write()
            .insert(report.run_id, report.clone());
        Ok(report)
    }

    async fn get_anomaly_report(&self, run_id: &RunId) -> Result<Option<AnomalyReport>> {
        Ok(self.anomaly_reports.read().get(run_id).cloned())
    }

    async fn list_anomaly_reports(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<AnomalyReport>> {
        let ids = self
            .anomaly_reports_by_asset
            .read()
            .get(asset_id)
            .cloned()
            .unwrap_or_default();
        let map = self.anomaly_reports.read();
        let mut items: Vec<_> = ids
            .into_iter()
            .rev()
            .filter_map(|id| map.get(&id).cloned())
            .collect();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn save_incident(&self, incident: Incident) -> Result<Incident> {
        if !self.incidents.read().contains_key(&incident.id) {
            self.incidents_order.write().push(incident.id);
        }
        self.incidents.write().insert(incident.id, incident.clone());
        Ok(incident)
    }

    async fn get_incident(&self, id: &IncidentId) -> Result<Option<Incident>> {
        Ok(self.incidents.read().get(id).cloned())
    }

    async fn list_incidents(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<Incident>> {
        let order = self.incidents_order.read().clone();
        let map = self.incidents.read();
        let mut items: Vec<_> = order
            .into_iter()
            .rev()
            .filter_map(|id| map.get(&id).cloned())
            .filter(|i| asset_id.map(|a| i.asset_id == *a).unwrap_or(true))
            .collect();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn append_incident_event(
        &self,
        event: IncidentTimelineEvent,
    ) -> Result<IncidentTimelineEvent> {
        self.incident_events
            .write()
            .entry(event.incident_id)
            .or_default()
            .push(event.clone());
        // Keep timeline cache on incident in sync when present.
        if let Some(inc) = self.incidents.write().get_mut(&event.incident_id) {
            inc.timeline.push(event.clone());
            inc.updated_at = event.at;
        }
        Ok(event)
    }

    async fn list_incident_events(
        &self,
        incident_id: &IncidentId,
        limit: Option<usize>,
    ) -> Result<Vec<IncidentTimelineEvent>> {
        let mut items = self
            .incident_events
            .read()
            .get(incident_id)
            .cloned()
            .unwrap_or_default();
        if let Some(n) = limit {
            if items.len() > n {
                items = items[items.len() - n..].to_vec();
            }
        }
        Ok(items)
    }

    async fn upsert_job(&self, job: JobDefinition) -> Result<JobDefinition> {
        self.jobs.write().insert(job.id, job.clone());
        Ok(job)
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<JobDefinition>> {
        Ok(self.jobs.read().get(id).cloned())
    }

    async fn list_jobs(&self) -> Result<Vec<JobDefinition>> {
        Ok(self.jobs.read().values().cloned().collect())
    }

    async fn save_job_run(&self, run: JobRun) -> Result<JobRun> {
        self.job_runs_by_job
            .write()
            .entry(run.job_id)
            .or_default()
            .push(run.id);
        self.job_runs.write().insert(run.id, run.clone());
        Ok(run)
    }

    async fn get_job_run(&self, id: &RunId) -> Result<Option<JobRun>> {
        Ok(self.job_runs.read().get(id).cloned())
    }

    async fn list_job_runs(&self, job_id: &JobId, limit: Option<usize>) -> Result<Vec<JobRun>> {
        let ids = self
            .job_runs_by_job
            .read()
            .get(job_id)
            .cloned()
            .unwrap_or_default();
        let runs_map = self.job_runs.read();
        let mut items: Vec<_> = ids
            .into_iter()
            .filter_map(|id| runs_map.get(&id).cloned())
            .collect();
        items.reverse();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }

    async fn upsert_rule_suggestion(&self, suggestion: RuleSuggestion) -> Result<RuleSuggestion> {
        let id = suggestion.id;
        let mut map = self.rule_suggestions.write();
        let is_new = !map.contains_key(&id);
        map.insert(id, suggestion.clone());
        drop(map);
        if is_new {
            self.rule_suggestions_order.write().push(id);
        }
        Ok(suggestion)
    }

    async fn get_rule_suggestion(&self, id: &SuggestionId) -> Result<Option<RuleSuggestion>> {
        Ok(self.rule_suggestions.read().get(id).cloned())
    }

    async fn list_rule_suggestions(
        &self,
        asset_id: Option<&AssetId>,
        status: Option<RuleSuggestionStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<RuleSuggestion>> {
        let order = self.rule_suggestions_order.read().clone();
        let map = self.rule_suggestions.read();
        let mut items: Vec<_> = order
            .into_iter()
            .rev()
            .filter_map(|id| map.get(&id).cloned())
            .filter(|s| asset_id.map(|a| s.asset_id == *a).unwrap_or(true))
            .filter(|s| status.map(|st| s.status == st).unwrap_or(true))
            .collect();
        if let Some(n) = limit {
            items.truncate(n);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drp_common::{AssetKind, SourceLocation};

    #[tokio::test]
    async fn asset_roundtrip() {
        let store = MemoryStore::new();
        let asset = Asset::new(
            "demo.public.orders",
            "orders",
            AssetKind::Table,
            SourceLocation::new("mock", "mock://orders"),
        );
        let id = asset.id;
        store.upsert_asset(asset).await.unwrap();
        assert_eq!(
            store.get_asset(&id).await.unwrap().unwrap().fqn,
            "demo.public.orders"
        );
    }
}
