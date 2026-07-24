//! Additional unit-style tests for validator plugins (crate integration tests).

use drp_common::{AssetKind, SourceLocation};
use drp_connectors::row;
use drp_core::{Asset, CheckDefinition, PluginContext, ValidatorPlugin};
use drp_validation::{NotNullValidator, RegexValidator, UniqueValidator};
use serde_json::json;

fn asset() -> Asset {
    Asset::new(
        "t.a",
        "a",
        AssetKind::Table,
        SourceLocation::new("mock", "m://"),
    )
}

#[tokio::test]
async fn not_null_passes_when_present() {
    let v = NotNullValidator::new();
    let check = CheckDefinition::new("c", asset().id, "not_null").with_param("column", json!("x"));
    let rows = vec![row(&[("x", json!("ok"))])];
    let r = v
        .validate(&check, &asset(), &rows, &PluginContext::new())
        .await
        .unwrap();
    assert!(matches!(r.status, drp_common::ValidationStatus::Passed));
}

#[tokio::test]
async fn unique_detects_duplicates() {
    let v = UniqueValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "unique").with_param("column", json!("x"));
    let rows = vec![row(&[("x", json!(1))]), row(&[("x", json!(1))])];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert!(matches!(r.status, drp_common::ValidationStatus::Failed));
}

#[tokio::test]
async fn regex_validates_email_shape() {
    let v = RegexValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "regex")
        .with_param("column", json!("email"))
        .with_param("pattern", json!(r"^[^@]+@[^@]+\.[^@]+$"));
    let rows = vec![
        row(&[("email", json!("ok@example.com"))]),
        row(&[("email", json!("bad"))]),
    ];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert!(matches!(r.status, drp_common::ValidationStatus::Failed));
}
