use std::sync::Arc;

use drp_common::{AssetKind, SourceLocation};
use drp_connectors::register_builtin_connectors;
use drp_core::{Asset, EventBus, PluginRegistry};
use drp_profiling::{register_builtin_profilers, ProfilingService};
use drp_storage::{MemoryStore, Store};

#[tokio::test]
async fn history_and_compare_over_time() {
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let plugins = PluginRegistry::new();
    register_builtin_connectors(&plugins);
    register_builtin_profilers(&plugins);

    let asset = Asset::new(
        "mock.public.orders",
        "orders",
        AssetKind::Table,
        SourceLocation::new("mock", "mock://"),
    );
    let id = asset.id;
    store.upsert_asset(asset).await.unwrap();

    let svc = ProfilingService::new(store.clone(), plugins, EventBus::new(), 100);
    let p1 = svc.profile_asset(&id, "mock", None).await.unwrap();
    let p2 = svc.profile_asset(&id, "mock", None).await.unwrap();
    assert_ne!(p1.run_id, p2.run_id);

    let history = svc.profile_history(&id, Some(10)).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].run_id, p2.run_id); // newest first

    let diff = svc.compare_profiles(&id, None, None).await.unwrap();
    assert_eq!(diff.current_run_id, p2.run_id);
    assert_eq!(diff.baseline_run_id, p1.run_id);
    assert_eq!(diff.row_count_delta, 0);
}
