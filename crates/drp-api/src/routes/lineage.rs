//! Lineage routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use drp_core::LineageEdge;
use drp_lineage::LineageSnapshot;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DepthQuery {
    depth: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AddEdgeRequest {
    from: String,
    to: String,
    #[serde(default = "default_kind")]
    kind: String,
}

fn default_kind() -> String {
    "transforms".into()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/lineage", get(snapshot))
        .route("/v1/lineage/edges", post(add_edge))
        .route("/v1/lineage/assets/{id}/upstream", get(upstream))
        .route("/v1/lineage/assets/{id}/downstream", get(downstream))
}

async fn snapshot(State(state): State<AppState>) -> Json<LineageSnapshot> {
    Json(state.lineage.snapshot())
}

async fn add_edge(
    State(state): State<AppState>,
    Json(body): Json<AddEdgeRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let from = body.from.parse().map_err(ApiError::from)?;
    let to = body.to.parse().map_err(ApiError::from)?;
    let mut edge = LineageEdge::transforms(from, to);
    edge.kind = body.kind;
    state.lineage.add_edge(edge);
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn upstream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DepthQuery>,
) -> ApiResult<Json<LineageSnapshot>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.lineage.upstream(&asset_id, q.depth)))
}

async fn downstream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DepthQuery>,
) -> ApiResult<Json<LineageSnapshot>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.lineage.downstream(&asset_id, q.depth)))
}
