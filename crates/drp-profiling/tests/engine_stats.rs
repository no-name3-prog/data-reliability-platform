use drp_common::{AssetKind, SourceLocation};
use drp_connectors::row;
use drp_core::{Asset, PluginContext, ProfilerPlugin, SemanticType};
use drp_profiling::BasicProfiler;
use serde_json::json;

#[tokio::test]
async fn profiles_null_unique_min_max_avg_histogram_and_email() {
    let p = BasicProfiler::new();
    let asset = Asset::new(
        "demo.public.users",
        "users",
        AssetKind::Table,
        SourceLocation::new("mock", "mock://"),
    );
    let rows = vec![
        row(&[
            ("email", json!("a@example.com")),
            ("amount", json!(10.0)),
            ("note", json!(null)),
        ]),
        row(&[
            ("email", json!("b@example.com")),
            ("amount", json!(20.0)),
            ("note", json!("x")),
        ]),
        row(&[
            ("email", json!("c@example.com")),
            ("amount", json!(30.0)),
            ("note", json!("y")),
        ]),
        row(&[
            ("email", json!("bad")),
            ("amount", json!(40.0)),
            ("note", json!(null)),
        ]),
    ];

    let profile = p
        .profile(&asset, &rows, &PluginContext::new())
        .await
        .unwrap();
    assert_eq!(profile.row_count, 4);

    let email = profile.columns.iter().find(|c| c.name == "email").unwrap();
    assert_eq!(email.semantic_type, SemanticType::Email);
    assert!(email.semantic_confidence >= 0.7);
    assert_eq!(email.distinct_count, 4);

    let amount = profile.columns.iter().find(|c| c.name == "amount").unwrap();
    assert_eq!(amount.min, Some(json!(10.0)));
    assert_eq!(amount.max, Some(json!(40.0)));
    assert!((amount.average.unwrap() - 25.0).abs() < 1e-9);
    assert!(!amount.histogram.is_empty());

    let note = profile.columns.iter().find(|c| c.name == "note").unwrap();
    assert_eq!(note.null_count, 2);
    assert!((note.null_percentage - 50.0).abs() < 1e-9);
}

#[tokio::test]
async fn detects_phone_and_date() {
    let p = BasicProfiler::new();
    let asset = Asset::new("t", "t", AssetKind::Table, SourceLocation::new("m", "m"));
    let rows = vec![
        row(&[
            ("phone", json!("+1 555-123-4567")),
            ("day", json!("2024-01-15")),
        ]),
        row(&[
            ("phone", json!("555-987-6543")),
            ("day", json!("2024-02-20")),
        ]),
        row(&[
            ("phone", json!("(020) 7946 0958")),
            ("day", json!("2023-12-01")),
        ]),
    ];
    let profile = p
        .profile(&asset, &rows, &PluginContext::new())
        .await
        .unwrap();
    let phone = profile.columns.iter().find(|c| c.name == "phone").unwrap();
    assert_eq!(phone.semantic_type, SemanticType::Phone);
    let day = profile.columns.iter().find(|c| c.name == "day").unwrap();
    assert_eq!(day.semantic_type, SemanticType::Date);
}
