//! Anomaly service — profile-history analysis, sample detectors, and incidents.

use std::sync::Arc;

use tracing::{info, instrument};

use drp_common::{AnomalyConfig, AssetId, Error, IncidentId, Result, RunId, UtcTimestamp};
use drp_core::{
    AnomalyReport, EventBus, Incident, IncidentStatus, PlatformEvent, PluginContext, PluginRegistry,
};
use drp_storage::Store;

use crate::engine::{ProfileAnomalyEngine, PROFILE_HISTORY_DETECTOR};

/// Orchestrates profile-history anomaly analysis and sample-based detectors.
#[derive(Clone)]
pub struct AnomalyService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    events: EventBus,
    sample_size: usize,
    default_detector: String,
    config: AnomalyConfig,
    engine: Arc<ProfileAnomalyEngine>,
}

impl AnomalyService {
    /// Create an anomaly service with default profile-history engine.
    pub fn new(
        store: Arc<dyn Store>,
        plugins: PluginRegistry,
        events: EventBus,
        sample_size: usize,
        config: AnomalyConfig,
    ) -> Self {
        Self {
            store,
            plugins,
            events,
            sample_size,
            default_detector: "null_spike".into(),
            config,
            engine: Arc::new(ProfileAnomalyEngine::with_defaults()),
        }
    }

    /// Access thresholds / toggles.
    pub fn config(&self) -> &AnomalyConfig {
        &self.config
    }

    /// Compare latest profile with historical profiles; open incidents on findings.
    ///
    /// Requires at least one stored profile. With only one profile, schema/row
    /// comparisons against a baseline are skipped but freshness is still evaluated.
    #[instrument(skip(self), fields(asset_id = %asset_id))]
    pub async fn analyze_profiles(&self, asset_id: &AssetId) -> Result<AnomalyReport> {
        let history = self
            .store
            .list_profile_history(asset_id, Some(self.config.history_window + 1))
            .await?;
        let current = history.first().cloned().ok_or_else(|| {
            Error::not_found(format!(
                "no profiles for asset {asset_id}; run profiling first"
            ))
        })?;
        // history[1..] are older profiles (newest-first list).
        let prior: Vec<_> = history.into_iter().skip(1).collect();

        let mut report = self.engine.analyze(&current, &prior, &self.config);
        report = self.persist_report_and_incidents(report).await?;
        Ok(report)
    }

    /// Run a sample-based detector plugin (legacy / complementary path).
    #[instrument(skip(self), fields(asset_id = %asset_id))]
    pub async fn detect(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
        detector_id: Option<&str>,
    ) -> Result<AnomalyReport> {
        let asset = self
            .store
            .get_asset(asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {asset_id}")))?;
        let connector = self.plugins.connector(connector_id)?;
        let detector_id = detector_id.unwrap_or(&self.default_detector);
        let detector = self.plugins.anomaly_detector(detector_id)?;
        let ctx = PluginContext::new();
        let rows = connector
            .sample_rows(&asset, self.sample_size, &ctx)
            .await?;
        let mut report = detector.detect(&asset, &rows, &ctx).await?;
        report = self.persist_report_and_incidents(report).await?;
        Ok(report)
    }

    /// List anomaly reports for an asset.
    pub async fn list_reports(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<AnomalyReport>> {
        self.store.list_anomaly_reports(asset_id, limit).await
    }

    /// Get a report by run id.
    pub async fn get_report(&self, run_id: &RunId) -> Result<AnomalyReport> {
        self.store
            .get_anomaly_report(run_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("anomaly report {run_id}")))
    }

    /// List incidents.
    pub async fn list_incidents(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<Incident>> {
        self.store.list_incidents(asset_id, limit).await
    }

    /// Get incident.
    pub async fn get_incident(&self, id: &IncidentId) -> Result<Incident> {
        self.store
            .get_incident(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("incident {id}")))
    }

    /// Update incident status.
    pub async fn set_incident_status(
        &self,
        id: &IncidentId,
        status: IncidentStatus,
    ) -> Result<Incident> {
        let mut incident = self.get_incident(id).await?;
        incident.status = status;
        incident.updated_at = UtcTimestamp::now();
        self.store.save_incident(incident).await
    }

    async fn persist_report_and_incidents(
        &self,
        mut report: AnomalyReport,
    ) -> Result<AnomalyReport> {
        let mut incident_ids = Vec::new();
        if self.config.create_incidents {
            for finding in &report.findings {
                let incident = Incident::from_finding(
                    report.asset_id,
                    report.run_id,
                    report.baseline_run_id,
                    report.current_run_id,
                    finding,
                );
                let id = incident.id;
                let saved = self.store.save_incident(incident).await?;
                self.events
                    .publish(PlatformEvent::IncidentOpened {
                        incident_id: saved.id,
                        asset_id: saved.asset_id,
                    })
                    .await;
                incident_ids.push(id);
            }
        }
        report.incident_ids = incident_ids;
        let saved = self.store.save_anomaly_report(report).await?;

        info!(
            run_id = %saved.run_id,
            findings = saved.findings.len(),
            incidents = saved.incident_ids.len(),
            detector = %saved.detector,
            "anomaly report saved"
        );

        self.events
            .publish(PlatformEvent::AnomalyReportCompleted {
                asset_id: saved.asset_id,
                run_id: saved.run_id,
                finding_count: saved.findings.len(),
            })
            .await;

        Ok(saved)
    }

    /// Detector id for the profile-history engine.
    pub fn profile_history_detector_id() -> &'static str {
        PROFILE_HISTORY_DETECTOR
    }
}
