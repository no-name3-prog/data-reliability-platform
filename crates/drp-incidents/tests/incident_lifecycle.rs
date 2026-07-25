//! Incident management lifecycle tests.

use std::sync::Arc;

use drp_common::{AssetId, CheckId, RunId};
use drp_core::{
    AnomalyFinding, AnomalyKind, AnomalySeverity, EventBus, IncidentStatus, PluginRegistry,
};
use drp_incidents::IncidentService;
use drp_notifications::NotificationService;
use drp_storage::{MemoryStore, Store};

fn svc() -> (Arc<dyn Store>, IncidentService) {
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let notifications = NotificationService::new(PluginRegistry::new(), vec!["log".into()], true);
    let incidents = IncidentService::new(store.clone(), EventBus::new(), notifications, true);
    (store, incidents)
}

#[tokio::test]
async fn validation_failure_opens_incident_with_timeline_and_owner() {
    let (_store, incidents) = svc();
    let asset = AssetId::new();
    let check = CheckId::new();
    let run = RunId::new();

    let opened = incidents
        .open_from_validation(
            asset,
            check,
            run,
            "not_null",
            drp_common::Severity::Error,
            "email not null failed",
            "column email has nulls",
            vec![asset],
        )
        .await
        .unwrap();

    assert_eq!(opened.status, IncidentStatus::Open);
    assert!(opened.affected_assets.contains(&asset));
    assert!(!opened.timeline.is_empty());

    let history = incidents.history(&opened.id, None).await.unwrap();
    assert!(!history.is_empty());
    assert!(history.iter().any(|e| e.event_type == "created"));

    let assigned = incidents
        .assign_owner(&opened.id, "owner@example.com", Some("admin".into()))
        .await
        .unwrap();
    assert_eq!(assigned.owner.as_deref(), Some("owner@example.com"));

    let hist2 = incidents.history(&opened.id, None).await.unwrap();
    assert!(hist2.iter().any(|e| e.event_type == "owner_assigned"));

    let resolved = incidents
        .set_status(
            &opened.id,
            IncidentStatus::Resolved,
            Some("owner@example.com".into()),
            Some("fixed upstream".into()),
        )
        .await
        .unwrap();
    assert_eq!(resolved.status, IncidentStatus::Resolved);
}

#[tokio::test]
async fn anomaly_finding_opens_incident() {
    let (_store, incidents) = svc();
    let asset = AssetId::new();
    let finding = AnomalyFinding::new(
        "profile_history",
        AnomalyKind::NullSpike,
        "null spike on email",
        AnomalySeverity::High,
    )
    .with_field("email");

    let opened = incidents
        .open_from_anomaly(asset, RunId::new(), None, None, &finding, vec![asset])
        .await
        .unwrap();
    assert_eq!(opened.field.as_deref(), Some("email"));
    assert!(!opened.timeline.is_empty());
}
