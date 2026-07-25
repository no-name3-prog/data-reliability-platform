//! Profile-history anomaly engine unit tests.

use drp_anomaly::{AnomalyService, ProfileAnomalyEngine, PROFILE_HISTORY_DETECTOR};
use drp_common::{AnomalyConfig, AssetId, DataType, RunId, UtcTimestamp};
use drp_core::{
    AnomalyKind, ColumnProfile, DatasetProfile, EventBus, PluginRegistry, SemanticType,
};
use drp_storage::{MemoryStore, Store};
use std::sync::Arc;

fn col(
    name: &str,
    null_pct: f64,
    unique: f64,
    distinct: u64,
    avg: Option<f64>,
    std: Option<f64>,
) -> ColumnProfile {
    ColumnProfile {
        name: name.into(),
        data_type: DataType::Float,
        semantic_type: SemanticType::Numeric,
        semantic_confidence: 0.9,
        null_count: 0,
        null_percentage: null_pct,
        distinct_count: distinct,
        unique_ratio: unique,
        min: None,
        max: None,
        average: avg,
        stddev: std,
        histogram: vec![],
        stats: Default::default(),
    }
}

fn profile(asset: AssetId, rows: u64, columns: Vec<ColumnProfile>) -> DatasetProfile {
    DatasetProfile {
        run_id: RunId::new(),
        asset_id: asset,
        asset_fqn: Some("t.a".into()),
        profiler: Some("basic".into()),
        connector: Some("mock".into()),
        sample_size: Some(rows),
        row_count: rows,
        columns,
        profiled_at: UtcTimestamp::now(),
    }
}

#[test]
fn detects_schema_add_remove_and_type_change() {
    let asset = AssetId::new();
    let base = profile(
        asset,
        100,
        vec![
            col("a", 0.0, 1.0, 100, Some(1.0), Some(0.1)),
            ColumnProfile {
                name: "b".into(),
                data_type: DataType::Integer,
                semantic_type: SemanticType::IntegerId,
                semantic_confidence: 0.8,
                null_count: 0,
                null_percentage: 0.0,
                distinct_count: 10,
                unique_ratio: 0.1,
                min: None,
                max: None,
                average: Some(5.0),
                stddev: Some(1.0),
                histogram: vec![],
                stats: Default::default(),
            },
        ],
    );
    let current = profile(
        asset,
        100,
        vec![
            ColumnProfile {
                name: "a".into(),
                data_type: DataType::String,
                semantic_type: SemanticType::Text,
                semantic_confidence: 0.5,
                null_count: 0,
                null_percentage: 0.0,
                distinct_count: 100,
                unique_ratio: 1.0,
                min: None,
                max: None,
                average: None,
                stddev: None,
                histogram: vec![],
                stats: Default::default(),
            },
            col("c", 0.0, 1.0, 5, Some(2.0), Some(0.2)),
        ],
    );

    let engine = ProfileAnomalyEngine::with_defaults();
    let report = engine.analyze(&current, &[base], &AnomalyConfig::default());
    assert_eq!(report.detector, PROFILE_HISTORY_DETECTOR);
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&AnomalyKind::SchemaChange));
}

#[test]
fn detects_row_count_drop_null_duplicate_distribution() {
    let asset = AssetId::new();
    let base = profile(
        asset,
        1000,
        vec![col("amount", 5.0, 0.95, 950, Some(100.0), Some(10.0))],
    );
    let current = profile(
        asset,
        400,
        vec![col("amount", 25.0, 0.50, 200, Some(200.0), Some(10.0))],
    );
    let engine = ProfileAnomalyEngine::with_defaults();
    let cfg = AnomalyConfig {
        freshness_max_age_secs: u64::MAX,
        ..AnomalyConfig::default()
    };
    let report = engine.analyze(&current, &[base], &cfg);
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&AnomalyKind::RowCountDrop), "{kinds:?}");
    assert!(kinds.contains(&AnomalyKind::NullSpike), "{kinds:?}");
    assert!(kinds.contains(&AnomalyKind::DuplicateSpike), "{kinds:?}");
    assert!(
        kinds.contains(&AnomalyKind::DistributionChange),
        "{kinds:?}"
    );
}

#[tokio::test]
async fn service_opens_incidents_from_profile_history() {
    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
    let asset = AssetId::new();
    let base = profile(
        asset,
        1000,
        vec![col("x", 0.0, 1.0, 1000, Some(10.0), Some(1.0))],
    );
    let current = profile(
        asset,
        200,
        vec![col("x", 40.0, 0.4, 80, Some(50.0), Some(1.0))],
    );
    store.save_profile(base).await.unwrap();
    store.save_profile(current).await.unwrap();

    let cfg = AnomalyConfig {
        freshness_max_age_secs: u64::MAX,
        create_incidents: true,
        ..AnomalyConfig::default()
    };

    let svc = AnomalyService::new(store, PluginRegistry::new(), EventBus::new(), 1000, cfg);
    let report = svc.analyze_profiles(&asset).await.unwrap();
    assert!(report.has_findings());
    assert!(!report.incident_ids.is_empty());
    let incidents = svc.list_incidents(Some(&asset), None).await.unwrap();
    assert_eq!(incidents.len(), report.incident_ids.len());
    assert!(incidents
        .iter()
        .all(|i| matches!(i.status, drp_core::IncidentStatus::Open)));
}
