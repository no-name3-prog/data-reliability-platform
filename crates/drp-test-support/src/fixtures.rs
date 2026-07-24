//! Golden fixtures and regression data builders.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde_json::{json, Value};

use drp_common::DataType;
use drp_connectors::{row, FixtureTable};
use drp_core::ColumnMeta;

/// Resolve a fixture path under `crates/drp-tests/fixtures`.
pub fn fixture_path(name: &str) -> PathBuf {
    // When running via cargo from workspace root (container /workspace).
    let candidates = [
        PathBuf::from(format!("crates/drp-tests/fixtures/{name}")),
        PathBuf::from(format!("fixtures/{name}")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../drp-tests/fixtures")
            .join(name),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(format!("crates/drp-tests/fixtures/{name}")))
}

/// Load a JSON fixture file.
pub fn load_json_fixture(name: &str) -> Value {
    let path = fixture_path(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("fixture json")
}

/// Canonical orders table used by regression suite (stable row counts).
pub fn regression_orders_fixture() -> FixtureTable {
    FixtureTable::new("regression.public.orders", "orders")
        .with_columns(vec![
            ColumnMeta::new("order_id", DataType::Integer)
                .required()
                .at(0),
            ColumnMeta::new("customer_email", DataType::String).at(1),
            ColumnMeta::new("amount", DataType::Float).at(2),
            ColumnMeta::new("status", DataType::String).at(3),
        ])
        .with_rows(vec![
            row(&[
                ("order_id", json!(100)),
                ("customer_email", json!("a@example.com")),
                ("amount", json!(10.0)),
                ("status", json!("paid")),
            ]),
            row(&[
                ("order_id", json!(101)),
                ("customer_email", json!("b@example.com")),
                ("amount", json!(20.5)),
                ("status", json!("paid")),
            ]),
            row(&[
                ("order_id", json!(102)),
                ("customer_email", json!(null)),
                ("amount", json!(5.0)),
                ("status", json!("pending")),
            ]),
        ])
}

/// Parameters for the well-known not-null check on customer_email.
pub fn orders_null_email_check() -> IndexMap<String, Value> {
    let mut m = IndexMap::new();
    m.insert("column".into(), json!("customer_email"));
    m
}

/// Ensure path exists helper for docs.
pub fn fixtures_dir() -> PathBuf {
    fixture_path(".")
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf()
}
