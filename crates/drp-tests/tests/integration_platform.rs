//! Integration tests: multi-crate flows (discover → profile → validate → lineage).

use drp_common::{SourceLocation, ValidationStatus};
use drp_core::{CheckDefinition, JobDefinition, LineageEdge};
use drp_test_support::PlatformHarness;
use serde_json::json;

#[tokio::test]
async fn integration_discover_profile_validate_lineage() {
    let h = PlatformHarness::new();

    let assets = h
        .metadata
        .discover_and_register("mock", SourceLocation::new("mock", "mock://local"))
        .await
        .expect("discover");
    assert!(assets.len() >= 2, "expected mock assets");

    let orders = assets
        .iter()
        .find(|a| a.name == "orders")
        .expect("orders asset");

    h.lineage.register_asset(orders.id, orders.fqn.clone());
    let users = assets.iter().find(|a| a.name == "users").unwrap();
    h.lineage.register_asset(users.id, users.fqn.clone());
    h.lineage
        .add_edge(LineageEdge::transforms(users.id, orders.id));

    let profile = h
        .profiling
        .profile_asset(&orders.id, "mock", Some("basic"))
        .await
        .expect("profile");
    assert_eq!(profile.row_count, 4);
    assert!(profile.columns.iter().any(|c| c.name == "customer_email"));

    let check = CheckDefinition::new("email not null", orders.id, "not_null")
        .with_param("column", json!("customer_email"));
    let check = h.validation.upsert_check(check).await.unwrap();
    let result = h.validation.run_check(&check.id, "mock").await.unwrap();
    assert_eq!(result.status, ValidationStatus::Failed);
    assert!(result.message.contains("null"));

    let down = h.lineage.downstream(&users.id, Some(5));
    assert!(down.nodes.len() >= 2);
    assert!(!down.edges.is_empty());
}

#[tokio::test]
async fn integration_scheduler_noop_job() {
    let h = PlatformHarness::new();
    let job = h
        .scheduler
        .upsert_job(JobDefinition::new("smoke", "noop"))
        .await
        .unwrap();
    let run = h.scheduler.run_job(&job.id).await.unwrap();
    assert!(run.error.is_none());
    assert!(matches!(run.status, drp_core::JobStatus::Succeeded));
}

#[tokio::test]
async fn integration_failing_connector_surfaces_error() {
    use drp_connectors::FailingConnector;
    use std::sync::Arc;

    let h = PlatformHarness::new();
    h.plugins
        .register_connector(Arc::new(FailingConnector::new("offline")));
    let err = h
        .metadata
        .discover_and_register("failing", SourceLocation::new("failing", "x://"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "connector_error");
}
