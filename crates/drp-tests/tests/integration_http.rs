//! Integration tests: HTTP API surface via in-process axum router.

use drp_test_support::{get_json, post_json, TestClient, TestPlatform};
use serde_json::json;

#[tokio::test]
async fn integration_health_ready_metrics() {
    let platform = TestPlatform::new().await;
    let client = TestClient::new(platform.router());

    let (st, body) = get_json(&client, "/livez").await;
    assert!(st.is_success());
    assert_eq!(body["status"], "ok");
    assert_eq!(body["container_first"], true);

    let (st, body) = get_json(&client, "/readyz").await;
    assert!(st.is_success(), "readyz body={body}");
    assert_eq!(body["ready"], true);

    let (st, body) = get_json(&client, "/metrics").await;
    assert!(st.is_success());
    // Prometheus text or empty renderer string
    let _ = body;
}

#[tokio::test]
async fn integration_discover_and_list_assets() {
    let platform = TestPlatform::new().await;
    let client = TestClient::new(platform.router());

    let (st, body) = post_json(
        &client,
        "/v1/assets/discover",
        &json!({"connector": "mock", "uri": "mock://local"}),
    )
    .await;
    assert!(st.is_success(), "discover={body}");
    assert!(body["count"].as_u64().unwrap_or(0) >= 2);

    let (st, body) = get_json(&client, "/v1/assets").await;
    assert!(st.is_success());
    assert!(body["count"].as_u64().unwrap_or(0) >= 2);
}

#[tokio::test]
async fn integration_plugins_endpoint_lists_builtins() {
    let platform = TestPlatform::new().await;
    let client = TestClient::new(platform.router());
    let (st, body) = get_json(&client, "/v1/plugins").await;
    assert!(st.is_success());
    assert!(body["count"].as_u64().unwrap_or(0) >= 4);
}
