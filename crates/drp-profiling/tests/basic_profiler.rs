use drp_common::{AssetKind, SourceLocation};
use drp_connectors::row;
use drp_core::{Asset, PluginContext, ProfilerPlugin};
use drp_profiling::BasicProfiler;
use serde_json::json;

#[tokio::test]
async fn basic_profiler_computes_null_and_distinct() {
    let p = BasicProfiler::new();
    let asset = Asset::new(
        "t.x",
        "x",
        AssetKind::Table,
        SourceLocation::new("mock", "m://"),
    );
    let rows = vec![
        row(&[("a", json!(1)), ("b", json!("x"))]),
        row(&[("a", json!(2)), ("b", json!(null))]),
        row(&[("a", json!(1)), ("b", json!("y"))]),
    ];
    let profile = p
        .profile(&asset, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(profile.row_count, 3);
    let b = profile.columns.iter().find(|c| c.name == "b").unwrap();
    assert_eq!(b.null_count, 1);
    assert_eq!(b.distinct_count, 2);
}
