//! Regression tests: golden expectations for DQ + profiling on fixed fixtures.
//!
//! Named `regression_*` so nextest profile `regression` can select them.

use std::sync::Arc;

use drp_common::{SourceLocation, ValidationStatus};
use drp_connectors::FixtureConnector;
use drp_core::{CheckDefinition, PluginContext};
use drp_test_support::{
    load_json_fixture, orders_null_email_check, regression_orders_fixture, PlatformHarness,
};
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test]
async fn regression_orders_profile_null_counts() {
    let h = PlatformHarness::new();
    let fixture = Arc::new(FixtureConnector::new());
    fixture.upsert_table(regression_orders_fixture());
    h.plugins.register_connector(fixture);

    // Manually register asset via fixture discover
    let assets = h
        .metadata
        .discover_and_register("fixture", SourceLocation::new("fixture", "fixture://"))
        .await
        .unwrap();
    let orders = assets.iter().find(|a| a.name == "orders").unwrap();

    let profile = h
        .profiling
        .profile_asset(&orders.id, "fixture", Some("basic"))
        .await
        .unwrap();

    let expected = load_json_fixture("orders_profile_expected.json");
    assert_eq!(profile.row_count, expected["row_count"].as_u64().unwrap());

    let email = profile
        .columns
        .iter()
        .find(|c| c.name == "customer_email")
        .expect("email column");
    assert_eq!(
        email.null_count,
        expected["customer_email_null_count"].as_u64().unwrap()
    );

    let col_names: Vec<_> = profile.columns.iter().map(|c| c.name.as_str()).collect();
    let expected_cols: Vec<_> = expected["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for c in expected_cols {
        assert!(col_names.contains(&c), "missing column {c}");
    }
}

#[tokio::test]
async fn regression_not_null_check_fails_on_known_null() {
    let h = PlatformHarness::new();
    let fixture = Arc::new(FixtureConnector::new());
    fixture.upsert_table(regression_orders_fixture());
    h.plugins.register_connector(fixture);

    let assets = h
        .metadata
        .discover_and_register("fixture", SourceLocation::new("fixture", "fixture://"))
        .await
        .unwrap();
    let orders = assets.iter().find(|a| a.name == "orders").unwrap();

    let mut check = CheckDefinition::new("regression email not null", orders.id, "not_null");
    check.params = orders_null_email_check();
    let check = h.validation.upsert_check(check).await.unwrap();
    let result = h.validation.run_check(&check.id, "fixture").await.unwrap();

    // Golden: still exactly one null → must fail
    assert_eq!(result.status, ValidationStatus::Failed);
    let nulls = result.metrics.get("null_count").and_then(|v| v.as_u64());
    assert_eq!(nulls, Some(1));
}

#[tokio::test]
async fn regression_unique_order_id_passes() {
    let h = PlatformHarness::new();
    let fixture = Arc::new(FixtureConnector::new());
    fixture.upsert_table(regression_orders_fixture());
    h.plugins.register_connector(fixture);

    let assets = h
        .metadata
        .discover_and_register("fixture", SourceLocation::new("fixture", "fixture://"))
        .await
        .unwrap();
    let orders = assets.iter().find(|a| a.name == "orders").unwrap();

    let check = CheckDefinition::new("order_id unique", orders.id, "unique")
        .with_param("column", json!("order_id"));
    let check = h.validation.upsert_check(check).await.unwrap();
    let result = h.validation.run_check(&check.id, "fixture").await.unwrap();
    assert_eq!(result.status, ValidationStatus::Passed);
}

#[tokio::test]
async fn regression_empty_fixture_discovers_nothing() {
    use drp_core::ConnectorPlugin;

    let empty = FixtureConnector::new();
    assert_eq!(empty.table_count(), 0);
    let assets = empty
        .discover(
            &SourceLocation::new("fixture", "fixture://empty"),
            &PluginContext::new(),
        )
        .await
        .unwrap();
    assert!(assets.is_empty());
}
