//! HTTP helpers for axum integration tests (in-process, no TCP).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tower::ServiceExt;

/// Thin wrapper around an axum [`Router`] for tests.
pub struct TestClient {
    router: Router,
}

impl TestClient {
    /// Wrap a router.
    pub fn new(router: Router) -> Self {
        Self { router }
    }
}

/// GET JSON helper.
pub async fn get_json(client: &TestClient, path: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let res = client.router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

/// POST JSON helper.
pub async fn post_json(client: &TestClient, path: &str, body: &Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    let res = client.router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Deserialize response body as typed JSON.
pub async fn response_json<T: DeserializeOwned>(
    client: &TestClient,
    method: &str,
    path: &str,
    body: Option<&Value>,
) -> (StatusCode, T) {
    let builder = Request::builder().method(method).uri(path);
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(b).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let res = client.router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let parsed: T = serde_json::from_slice(&bytes).expect("typed json body");
    (status, parsed)
}
