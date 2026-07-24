use drp_common::SourceLocation;
use drp_connectors::CsvConnector;
use drp_core::{ConnectorPlugin, PluginContext};

#[tokio::test]
async fn csv_discovers_orders() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata");
    let c = CsvConnector::new();
    let loc = SourceLocation::new("csv", path);
    c.test_connection(&loc, &PluginContext::new())
        .await
        .unwrap();
    let tree = c
        .discover_catalog(&loc, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(tree.table_count(), 1);
    let t = tree.all_tables()[0];
    assert_eq!(t.name, "orders");
    assert!(t.columns.iter().any(|c| c.name == "order_id"));

    let assets = c.discover(&loc, &PluginContext::new()).await.unwrap();
    let rows = c
        .sample_rows(&assets[0], 10, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(rows.len(), 4);
}
