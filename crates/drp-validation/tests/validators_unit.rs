//! Unit-style tests for all built-in validator plugins.

use drp_common::{AssetKind, SourceLocation, ValidationStatus};
use drp_connectors::row;
use drp_core::{Asset, CheckDefinition, PluginContext, ValidatorPlugin};
use drp_validation::{
    AcceptedValuesValidator, FreshnessValidator, NotNullValidator, RangeValidator,
    ReferentialIntegrityValidator, RegexValidator, RowCountValidator, UniqueValidator,
};
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
    assert!(matches!(r.status, ValidationStatus::Passed));
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
    assert!(matches!(r.status, ValidationStatus::Failed));
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
    assert!(matches!(r.status, ValidationStatus::Failed));
}

#[tokio::test]
async fn accepted_values_rejects_unknown() {
    let v = AcceptedValuesValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "accepted_values")
        .with_param("column", json!("status"))
        .with_param("values", json!(["open", "closed"]));
    let rows = vec![
        row(&[("status", json!("open"))]),
        row(&[("status", json!("weird"))]),
    ];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Failed);
}

#[tokio::test]
async fn range_enforces_bounds() {
    let v = RangeValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "range")
        .with_param("column", json!("amount"))
        .with_param("min", json!(0))
        .with_param("max", json!(100));
    let rows = vec![
        row(&[("amount", json!(10))]),
        row(&[("amount", json!(150))]),
    ];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Failed);
    assert_eq!(r.metrics.get("out_of_range"), Some(&json!(1)));
}

#[tokio::test]
async fn row_count_min_fails() {
    let v = RowCountValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "row_count").with_param("min", json!(5));
    let rows = vec![row(&[("x", json!(1))])];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Failed);
}

#[tokio::test]
async fn row_count_passes() {
    let v = RowCountValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "row_count")
        .with_param("min", json!(1))
        .with_param("max", json!(10));
    let rows = vec![row(&[("x", json!(1))]), row(&[("x", json!(2))])];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Passed);
}

#[tokio::test]
async fn referential_integrity_with_values() {
    let v = ReferentialIntegrityValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "referential_integrity")
        .with_param("column", json!("user_id"))
        .with_param("values", json!([1, 2, 3]));
    let rows = vec![
        row(&[("user_id", json!(1))]),
        row(&[("user_id", json!(99))]),
    ];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Failed);
    assert_eq!(r.metrics.get("missing_count"), Some(&json!(1)));
}

#[tokio::test]
async fn freshness_passes_recent() {
    let v = FreshnessValidator::new();
    let a = asset();
    let now = chrono::Utc::now().to_rfc3339();
    let check = CheckDefinition::new("c", a.id, "freshness")
        .with_param("column", json!("ts"))
        .with_param("max_age_secs", json!(3600));
    let rows = vec![row(&[("ts", json!(now))])];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Passed);
}

#[tokio::test]
async fn freshness_fails_stale() {
    let v = FreshnessValidator::new();
    let a = asset();
    let check = CheckDefinition::new("c", a.id, "freshness")
        .with_param("column", json!("ts"))
        .with_param("max_age_secs", json!(60));
    let rows = vec![row(&[("ts", json!("2020-01-01T00:00:00Z"))])];
    let r = v
        .validate(&check, &a, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(r.status, ValidationStatus::Failed);
}
