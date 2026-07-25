//! SQL lineage ingest and impact analysis.

use drp_common::AssetId;
use drp_core::LineageNodeKind;
use drp_lineage::LineageService;

#[test]
fn sql_create_view_builds_table_and_column_lineage() {
    let svc = LineageService::new(10);
    let sql = r#"
        CREATE VIEW analytics.orders_enriched AS
        SELECT o.id AS order_id, o.amount, u.email AS customer_email
        FROM raw.orders o
        JOIN raw.users u ON o.user_id = u.id
    "#;
    let res = svc.ingest_sql(sql, None).unwrap();
    assert!(res.table_edges_added >= 2);
    assert!(res.column_edges_added >= 2);
    assert!(res.target_asset_id.is_some());

    let target = res.target_asset_id.unwrap();
    let up = svc.upstream(&target, Some(5));
    assert!(up.nodes.len() >= 3);
    assert!(!up.edges.is_empty());

    let col_up = svc.column_upstream(&target, "customer_email", Some(3));
    assert!(
        !col_up.is_empty(),
        "expected column lineage for customer_email"
    );
}

#[test]
fn impact_lists_dashboards_pipelines_and_datasets() {
    let svc = LineageService::new(10);
    let orders = AssetId::new();
    let mart = AssetId::new();
    let dash = AssetId::new();
    let pipe = AssetId::new();
    let dataset = AssetId::new();

    svc.register_asset(orders, "raw.orders");
    svc.register_node(
        mart,
        "mart.orders",
        LineageNodeKind::Table,
        Some("mart.orders".into()),
    );
    svc.register_node(
        dataset,
        "orders_dataset",
        LineageNodeKind::Dataset,
        Some("dataset.orders".into()),
    );
    svc.add_edge(drp_core::LineageEdge::transforms(orders, mart));
    svc.add_edge(drp_core::LineageEdge::transforms(mart, dataset));
    svc.register_dashboard(dash, "Exec Dashboard", &[mart, dataset]);
    svc.register_pipeline(pipe, "daily_orders_etl", &[orders], &[mart]);

    let impact = svc.impact_table_change(&orders, Some(10));
    assert!(impact.total_affected() >= 3);
    assert!(!impact.dashboards.is_empty() || !impact.pipelines.is_empty());
    assert!(
        impact.datasets.iter().any(|d| d.asset_id == dataset)
            || impact.tables.iter().any(|t| t.asset_id == mart)
    );

    let v_impact = svc.impact_validation_failed(
        &orders,
        Some("check-1".into()),
        Some("not_null failed".into()),
        Some(10),
    );
    assert_eq!(
        match &v_impact.trigger {
            drp_core::ImpactTrigger::ValidationFailed { asset_id, .. } => *asset_id,
            _ => panic!("wrong trigger"),
        },
        orders
    );
    assert!(v_impact.total_affected() >= 1);
}

#[test]
fn insert_select_lineage() {
    let svc = LineageService::new(5);
    let res = svc
        .ingest_sql(
            "INSERT INTO mart.fact_orders SELECT id, amount FROM staging.orders",
            None,
        )
        .unwrap();
    assert_eq!(res.table_edges_added, 1);
    let target = res.target_asset_id.unwrap();
    let up = svc.upstream(&target, Some(2));
    assert!(up.nodes.len() >= 2);
}
