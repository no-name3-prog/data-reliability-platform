//! Health, liveness, readiness, and startup probes.
//!
//! | Endpoint | Use |
//! |----------|-----|
//! | `GET /health` | Generic liveness (process up) |
//! | `GET /livez` | Kubernetes liveness |
//! | `GET /readyz` | Kubernetes readiness |
//! | `GET /startupz` | Kubernetes startup |
//! | `GET /ready` | Alias for readiness (compat) |

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;

use crate::state::AppState;
use crate::VERSION;

#[derive(Serialize)]
struct ProbeOk {
    status: &'static str,
    product: &'static str,
    version: &'static str,
    environment: String,
    plugins: usize,
    container_first: bool,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    ready: bool,
    product: &'static str,
    version: &'static str,
    storage: &'static str,
    lineage_nodes: usize,
    lineage_edges: usize,
    plugins: usize,
    infra: InfraStatus,
}

#[derive(Serialize)]
struct InfraStatus {
    database_url_configured: bool,
    redis_url_configured: bool,
    s3_endpoint_configured: bool,
}

#[derive(Serialize)]
struct NotReady {
    status: &'static str,
    ready: bool,
    reason: String,
}

/// Mount probe routes (no auth).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(livez))
        .route("/livez", get(livez))
        .route("/startupz", get(livez))
        .route("/readyz", get(readyz))
        .route("/ready", get(readyz))
        .route("/v1/health", get(livez))
}

/// Liveness / startup: process is running and can serve trivial requests.
async fn livez(State(state): State<AppState>) -> Json<ProbeOk> {
    Json(ProbeOk {
        status: "ok",
        product: drp_common::PRODUCT_NAME,
        version: VERSION,
        environment: state.platform.environment().to_string(),
        plugins: state.platform.plugins.len(),
        container_first: true,
    })
}

/// Readiness: app can accept traffic (plugins registered, store accessible).
async fn readyz(
    State(state): State<AppState>,
) -> Result<Json<ReadyResponse>, (StatusCode, Json<NotReady>)> {
    if state.platform.plugins.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(NotReady {
                status: "not_ready",
                ready: false,
                reason: "no plugins registered".into(),
            }),
        ));
    }

    // Smoke the store with a bounded list call.
    if let Err(e) = state.store.list_assets(Some(1)).await {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(NotReady {
                status: "not_ready",
                ready: false,
                reason: format!("storage: {e}"),
            }),
        ));
    }

    let (nodes, edges) = state.lineage.stats().map_err(|e| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(NotReady {
                status: "not_ready",
                ready: false,
                reason: format!("lineage: {e}"),
            }),
        )
    })?;

    let infra = &state.platform.config.infra;
    Ok(Json(ReadyResponse {
        status: "ok",
        ready: true,
        product: drp_common::PRODUCT_NAME,
        version: VERSION,
        storage: "ok",
        lineage_nodes: nodes,
        lineage_edges: edges,
        plugins: state.platform.plugins.len(),
        infra: InfraStatus {
            database_url_configured: !infra.database_url.is_empty(),
            redis_url_configured: !infra.redis_url.is_empty(),
            s3_endpoint_configured: !infra.s3_endpoint.is_empty(),
        },
    }))
}
