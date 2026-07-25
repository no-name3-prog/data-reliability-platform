//! Suite execution history and scheduled validation job.

use std::sync::Arc;

use drp_common::{SourceLocation, ValidationStatus};
use drp_core::{CheckDefinition, EventBus, PluginRegistry};
use drp_scheduler::SchedulerService;
use drp_storage::{MemoryStore, Store};
use drp_validation::{
    register_builtin_validators, ValidationJobHandler, ValidationService, VALIDATION_JOB_KIND,
};
use serde_json::json;

fn build() -> (
    Arc<MemoryStore>,
    ValidationService,
    SchedulerService,
    PluginRegistry,
) {
    use drp_connectors::register_builtin_connectors;

    let store: Arc<MemoryStore> = Arc::new(MemoryStore::new());
    let store_trait: Arc<dyn drp_storage::Store> = store.clone();
    let plugins = PluginRegistry::new();
    register_builtin_connectors(&plugins);
    register_builtin_validators(&plugins);
    let events = EventBus::new();
    let validation = ValidationService::new(
        store_trait.clone(),
        plugins.clone(),
        events.clone(),
        10_000,
        false,
    );
    let scheduler = SchedulerService::new(store_trait, events, 4);
    scheduler
        .handlers()
        .register(Arc::new(ValidationJobHandler::new(validation.clone())));
    (store, validation, scheduler, plugins)
}

#[tokio::test]
async fn suite_run_saves_history_and_per_check_results() {
    let (store, validation, _, _) = build();

    // Register a mock asset via connector discover path simulation.
    use drp_common::AssetKind;
    use drp_core::Asset;
    let asset = Asset::new(
        "demo.public.orders",
        "orders",
        AssetKind::Table,
        SourceLocation::new("mock", "mock://orders"),
    );
    let asset_id = asset.id;
    store.upsert_asset(asset).await.unwrap();

    let c1 = validation
        .upsert_check(
            CheckDefinition::new("email nn", asset_id, "not_null")
                .with_param("column", json!("customer_email")),
        )
        .await
        .unwrap();
    let _c2 = validation
        .upsert_check(
            CheckDefinition::new("rows", asset_id, "row_count").with_param("min", json!(1)),
        )
        .await
        .unwrap();

    let run = validation
        .run_checks_for_asset(&asset_id, "mock")
        .await
        .unwrap();
    assert!(run.results.len() >= 2);
    assert_eq!(run.id, run.results[0].suite_run_id.unwrap());

    // History retained.
    let history = validation
        .list_validation_runs(Some(&asset_id), Some(10))
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, run.id);

    let per_check = validation
        .list_check_results(&c1.id, Some(5))
        .await
        .unwrap();
    assert_eq!(per_check.len(), 1);

    // Second suite appends history.
    let run2 = validation
        .run_checks_for_asset(&asset_id, "mock")
        .await
        .unwrap();
    assert_ne!(run2.id, run.id);
    let history = validation
        .list_validation_runs(Some(&asset_id), None)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn scheduled_validation_job_runs_suite() {
    let (store, validation, scheduler, _) = build();

    use drp_common::AssetKind;
    use drp_core::Asset;
    let asset = Asset::new(
        "demo.public.users",
        "users",
        AssetKind::Table,
        SourceLocation::new("mock", "mock://users"),
    );
    let asset_id = asset.id;
    store.upsert_asset(asset).await.unwrap();

    validation
        .upsert_check(
            CheckDefinition::new("id unique", asset_id, "unique").with_param("column", json!("id")),
        )
        .await
        .unwrap();

    let job = validation
        .schedule_asset_validation("dq users", asset_id, "mock", "*/5 * * * *", None)
        .await
        .unwrap();
    assert_eq!(job.kind, VALIDATION_JOB_KIND);

    let job_run = scheduler.run_job(&job.id).await.unwrap();
    assert!(job_run.error.is_none(), "{:?}", job_run.error);
    assert!(matches!(job_run.status, drp_core::JobStatus::Succeeded));
    let payload = job_run.result.expect("payload");
    assert!(payload.get("suite_run_id").is_some());

    let suites = validation
        .list_validation_runs(Some(&asset_id), None)
        .await
        .unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].job_id, Some(job.id));
}

#[tokio::test]
async fn single_check_history_appends() {
    let (store, validation, _, _) = build();
    use drp_common::AssetKind;
    use drp_core::Asset;
    let asset = Asset::new(
        "t",
        "t",
        AssetKind::Table,
        SourceLocation::new("mock", "mock://t"),
    );
    let asset_id = asset.id;
    store.upsert_asset(asset).await.unwrap();

    let check = validation
        .upsert_check(
            CheckDefinition::new("nn", asset_id, "not_null").with_param("column", json!("id")),
        )
        .await
        .unwrap();

    let r1 = validation.run_check(&check.id, "mock").await.unwrap();
    let r2 = validation.run_check(&check.id, "mock").await.unwrap();
    assert_ne!(r1.run_id, r2.run_id);
    // mock orders/users may fail not_null on missing column — either way status is set
    assert!(matches!(
        r1.status,
        ValidationStatus::Passed | ValidationStatus::Failed | ValidationStatus::Error
    ));

    let hist = validation
        .list_check_results(&check.id, None)
        .await
        .unwrap();
    assert_eq!(hist.len(), 2);
}
