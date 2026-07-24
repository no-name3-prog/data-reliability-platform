//! Health and readiness endpoints.

use axum::extract::State;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::VERSION;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    product: &'static str,
    version: &'static str,
    environment: String,
    plugins: usize,
    container_first: bool,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    storage: &'static str,
    lineage_nodes: usize,
    lineage_edges: usize,
    infra: InfraStatus,
}

#[derive(Serialize)]
struct InfraStatus {
    database_url_configured: bool,
    redis_url_configured: bool,
    s3_endpoint_configured: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/v1/health", get(health))
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        product: drp_common::PRODUCT_NAME,
        version: VERSION,
        environment: state.platform.environment().to_string(),
        plugins: state.platform.plugins.len(),
        container_first: true,
    })
}

async fn ready(State(state): State<AppState>) -> ApiResult<Json<ReadyResponse>> {
    let (nodes, edges) = state
        .lineage
        .stats()
        .map_err(crate::error::ApiError::from)?;
    let infra = &state.platform.config.infra;
    Ok(Json(ReadyResponse {
        ready: true,
        storage: "ok",
        lineage_nodes: nodes,
        lineage_edges: edges,
        infra: InfraStatus {
            database_url_configured: !infra.database_url.is_empty(),
            redis_url_configured: !infra.redis_url.is_empty(),
            s3_endpoint_configured: !infra.s3_endpoint.is_empty(),
        },
    }))
}
