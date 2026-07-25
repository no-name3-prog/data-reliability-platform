//! Incident management service.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::json;
use tracing::{info, instrument, warn};

use drp_common::{AssetId, CheckId, IncidentId, Result, RunId, Severity, UtcTimestamp};
use drp_core::{
    AnomalyFinding, AnomalySeverity, EventBus, Incident, IncidentSource, IncidentStatus,
    IncidentTimelineEvent, PlatformEvent,
};
use drp_notifications::NotificationService;
use drp_storage::Store;

/// Manages incident lifecycle, timeline history, and multi-channel notifications.
#[derive(Clone)]
pub struct IncidentService {
    store: Arc<dyn Store>,
    events: EventBus,
    notifications: NotificationService,
    /// When true, notify default channels on open / status change.
    notify_on_change: bool,
}

impl IncidentService {
    /// Create an incident service.
    pub fn new(
        store: Arc<dyn Store>,
        events: EventBus,
        notifications: NotificationService,
        notify_on_change: bool,
    ) -> Self {
        Self {
            store,
            events,
            notifications,
            notify_on_change,
        }
    }

    /// Persist a fully built incident (appends timeline + notifies).
    #[instrument(skip(self, incident), fields(incident_id = %incident.id))]
    pub async fn open(&self, mut incident: Incident) -> Result<Incident> {
        // Ensure timeline has created event.
        if incident.timeline.is_empty() {
            incident.timeline.push(
                IncidentTimelineEvent::new(incident.id, "created", "Incident opened")
                    .with_actor("system"),
            );
        }
        let events = incident.timeline.clone();
        let saved = self.store.save_incident(incident).await?;
        for ev in events {
            let _ = self.store.append_incident_event(ev).await;
        }

        if self.notify_on_change {
            match self.notify_incident(&saved, "opened").await {
                Ok(channels) => {
                    if !channels.is_empty() {
                        let mut updated = saved.clone();
                        updated.notified_channels = channels.clone();
                        let note = IncidentTimelineEvent::new(
                            updated.id,
                            "notification_sent",
                            format!("Notified channels: {}", channels.join(", ")),
                        )
                        .with_actor("system:notifications")
                        .with_detail("channels", json!(channels));
                        updated.timeline.push(note.clone());
                        let _ = self.store.append_incident_event(note).await;
                        let _ = self.store.save_incident(updated.clone()).await;
                        self.events
                            .publish(PlatformEvent::IncidentOpened {
                                incident_id: saved.id,
                                asset_id: saved.asset_id,
                            })
                            .await;
                        return Ok(updated);
                    }
                }
                Err(e) => warn!(error = %e, "incident notification failed"),
            }
        }

        self.events
            .publish(PlatformEvent::IncidentOpened {
                incident_id: saved.id,
                asset_id: saved.asset_id,
            })
            .await;
        info!(%saved.id, title = %saved.title, "incident opened");
        Ok(saved)
    }

    /// Open from anomaly finding (+ optional extra affected assets from lineage).
    pub async fn open_from_anomaly(
        &self,
        asset_id: AssetId,
        report_run_id: RunId,
        baseline_run_id: Option<RunId>,
        current_run_id: Option<RunId>,
        finding: &AnomalyFinding,
        affected_assets: Vec<AssetId>,
    ) -> Result<Incident> {
        let incident = Incident::from_anomaly_finding(
            asset_id,
            report_run_id,
            baseline_run_id,
            current_run_id,
            finding,
            affected_assets,
        );
        self.open(incident).await
    }

    /// Open from a failed validation check.
    pub async fn open_from_validation(
        &self,
        asset_id: AssetId,
        check_id: CheckId,
        result_run_id: RunId,
        validator: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
        message: impl Into<String>,
        affected_assets: Vec<AssetId>,
    ) -> Result<Incident> {
        let incident = Incident::from_validation_failure(
            asset_id,
            check_id,
            result_run_id,
            validator,
            AnomalySeverity::from(severity),
            title,
            message,
            affected_assets,
        );
        self.open(incident).await
    }

    /// Get incident (timeline hydrated from history store).
    pub async fn get(&self, id: &IncidentId) -> Result<Incident> {
        let mut incident = self
            .store
            .get_incident(id)
            .await?
            .ok_or_else(|| drp_common::Error::not_found(format!("incident {id}")))?;
        let history = self.store.list_incident_events(id, None).await?;
        if !history.is_empty() {
            incident.timeline = history;
        }
        Ok(incident)
    }

    /// List incidents.
    pub async fn list(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<Incident>> {
        self.store.list_incidents(asset_id, limit).await
    }

    /// Complete timeline history for an incident.
    pub async fn history(
        &self,
        id: &IncidentId,
        limit: Option<usize>,
    ) -> Result<Vec<IncidentTimelineEvent>> {
        // ensure exists
        let _ = self.get(id).await?;
        self.store.list_incident_events(id, limit).await
    }

    /// Assign owner; appends timeline event.
    pub async fn assign_owner(
        &self,
        id: &IncidentId,
        owner: impl Into<String>,
        actor: Option<String>,
    ) -> Result<Incident> {
        let owner = owner.into();
        let mut incident = self.get(id).await?;
        let prev = incident.owner.clone();
        incident.owner = Some(owner.clone());
        incident.updated_at = UtcTimestamp::now();
        let mut ev = IncidentTimelineEvent::new(
            incident.id,
            "owner_assigned",
            format!("Owner set to {owner}"),
        )
        .with_detail("owner", json!(owner))
        .with_detail("previous", json!(prev));
        if let Some(a) = actor {
            ev = ev.with_actor(a);
        }
        incident.timeline.push(ev.clone());
        let saved = self.store.save_incident(incident).await?;
        let _ = self.store.append_incident_event(ev).await;
        Ok(saved)
    }

    /// Update status; appends timeline; optional re-notify.
    pub async fn set_status(
        &self,
        id: &IncidentId,
        status: IncidentStatus,
        actor: Option<String>,
        note: Option<String>,
    ) -> Result<Incident> {
        let mut incident = self.get(id).await?;
        let prev = incident.status;
        incident.status = status;
        incident.updated_at = UtcTimestamp::now();
        let msg = note
            .unwrap_or_else(|| format!("Status changed {} → {}", prev.as_str(), status.as_str()));
        let mut ev = IncidentTimelineEvent::new(incident.id, "status_changed", msg)
            .with_detail("from", json!(prev.as_str()))
            .with_detail("to", json!(status.as_str()));
        if let Some(a) = actor.clone() {
            ev = ev.with_actor(a);
        }
        incident.timeline.push(ev.clone());
        let saved = self.store.save_incident(incident).await?;
        let _ = self.store.append_incident_event(ev).await;

        if self.notify_on_change {
            let _ = self
                .notify_incident(&saved, &format!("status:{}", status.as_str()))
                .await;
        }
        Ok(saved)
    }

    /// Add a free-form note to the timeline.
    pub async fn add_note(
        &self,
        id: &IncidentId,
        message: impl Into<String>,
        actor: Option<String>,
    ) -> Result<IncidentTimelineEvent> {
        let incident = self.get(id).await?;
        let mut ev = IncidentTimelineEvent::new(incident.id, "note", message)
            .with_detail("kind", json!("note"));
        if let Some(a) = actor {
            ev = ev.with_actor(a);
        }
        let saved = self.store.append_incident_event(ev).await?;
        // touch incident updated_at
        let mut inc = incident;
        inc.updated_at = UtcTimestamp::now();
        inc.timeline.push(saved.clone());
        let _ = self.store.save_incident(inc).await?;
        Ok(saved)
    }

    /// Update affected assets list.
    pub async fn set_affected_assets(
        &self,
        id: &IncidentId,
        assets: Vec<AssetId>,
        actor: Option<String>,
    ) -> Result<Incident> {
        let mut incident = self.get(id).await?;
        incident.affected_assets = assets.clone();
        if !incident.affected_assets.contains(&incident.asset_id) {
            incident.affected_assets.insert(0, incident.asset_id);
        }
        incident.updated_at = UtcTimestamp::now();
        let mut ev = IncidentTimelineEvent::new(
            incident.id,
            "affected_assets_updated",
            format!(
                "Affected assets set ({} assets)",
                incident.affected_assets.len()
            ),
        )
        .with_detail(
            "assets",
            json!(incident
                .affected_assets
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()),
        );
        if let Some(a) = actor {
            ev = ev.with_actor(a);
        }
        incident.timeline.push(ev.clone());
        let saved = self.store.save_incident(incident).await?;
        let _ = self.store.append_incident_event(ev).await;
        Ok(saved)
    }

    async fn notify_incident(&self, incident: &Incident, action: &str) -> Result<Vec<String>> {
        let mut meta = IndexMap::new();
        meta.insert("incident_id".into(), json!(incident.id.to_string()));
        meta.insert("asset_id".into(), json!(incident.asset_id.to_string()));
        meta.insert(
            "affected_assets".into(),
            json!(incident
                .affected_assets
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()),
        );
        meta.insert("severity".into(), json!(format!("{:?}", incident.severity)));
        meta.insert("status".into(), json!(incident.status.as_str()));
        meta.insert("action".into(), json!(action));
        meta.insert("owner".into(), json!(incident.owner));
        if let Some(ref d) = incident.detector {
            meta.insert("detector".into(), json!(d));
        }
        meta.insert(
            "source".into(),
            json!(match &incident.source {
                IncidentSource::Validation { .. } => "validation",
                IncidentSource::Anomaly { .. } => "anomaly",
                IncidentSource::Manual { .. } => "manual",
            }),
        );

        let subject = format!(
            "[DRP {}] {} — {}",
            action.to_uppercase(),
            incident.severity_label(),
            incident.title
        );
        let body = format!(
            "{}\n\nStatus: {}\nOwner: {}\nPrimary asset: {}\nAffected: {}\n",
            incident.message,
            incident.status.as_str(),
            incident.owner.as_deref().unwrap_or("(unassigned)"),
            incident.asset_id,
            incident
                .affected_assets
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.notifications.notify(&subject, &body, meta).await?;
        Ok(self.notifications.default_channels().to_vec())
    }
}

trait SeverityLabel {
    fn severity_label(&self) -> &'static str;
}

impl SeverityLabel for Incident {
    fn severity_label(&self) -> &'static str {
        match self.severity {
            AnomalySeverity::Low => "LOW",
            AnomalySeverity::Medium => "MEDIUM",
            AnomalySeverity::High => "HIGH",
            AnomalySeverity::Critical => "CRITICAL",
        }
    }
}
