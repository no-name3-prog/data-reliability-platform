//! Lineage routes: graph, SQL ingest, column lineage, impact analysis.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use drp_common::AssetId;
use drp_core::{ColumnLineageEdge, ImpactReport, LineageEdge, LineageNode, LineageNodeKind};
use drp_lineage::{LineageSnapshot, SqlIngestResult};

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

#[derive(Debug, Deserialize)]
pub struct ParseSqlRequest {
    /// SQL text (SELECT / CREATE VIEW|TABLE AS / INSERT SELECT).
    sql: String,
    /// Optional target asset id when SQL has no CREATE/INSERT target.
    #[serde(default)]
    target_asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterNodeRequest {
    /// Optional explicit id; generated if omitted.
    #[serde(default)]
    asset_id: Option<String>,
    name: String,
    #[serde(default)]
    fqn: Option<String>,
    kind: LineageNodeKind,
    /// Upstream assets this node reads (for dashboards/pipelines).
    #[serde(default)]
    reads: Vec<String>,
    /// Downstream assets a pipeline produces.
    #[serde(default)]
    produces: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationImpactRequest {
    asset_id: String,
    #[serde(default)]
    check_id: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    depth: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ColumnLineageResponse {
    items: Vec<ColumnLineageEdge>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/lineage", get(snapshot))
        .route("/v1/lineage/edges", post(add_edge))
        .route("/v1/lineage/nodes", post(register_node))
        .route("/v1/lineage/parse-sql", post(parse_sql))
        .route("/v1/lineage/assets/{id}/upstream", get(upstream))
        .route("/v1/lineage/assets/{id}/downstream", get(downstream))
        .route(
            "/v1/lineage/assets/{id}/columns/{column}/upstream",
            get(column_upstream),
        )
        .route(
            "/v1/lineage/assets/{id}/columns/{column}/downstream",
            get(column_downstream),
        )
        .route("/v1/lineage/assets/{id}/impact", get(table_impact))
        .route("/v1/lineage/impact/validation", post(validation_impact))
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

async fn register_node(
    State(state): State<AppState>,
    Json(body): Json<RegisterNodeRequest>,
) -> ApiResult<Json<LineageNode>> {
    let id = match body.asset_id {
        Some(s) => s.parse().map_err(ApiError::from)?,
        None => AssetId::new(),
    };
    let reads: Result<Vec<AssetId>, _> = body.reads.iter().map(|s| s.parse()).collect();
    let reads = reads.map_err(ApiError::from)?;
    let produces: Result<Vec<AssetId>, _> = body.produces.iter().map(|s| s.parse()).collect();
    let produces = produces.map_err(ApiError::from)?;

    match body.kind {
        LineageNodeKind::Dashboard => {
            state
                .lineage
                .register_dashboard(id, body.name.clone(), &reads);
        }
        LineageNodeKind::Pipeline => {
            state
                .lineage
                .register_pipeline(id, body.name.clone(), &reads, &produces);
        }
        other => {
            state
                .lineage
                .register_node(id, body.name.clone(), other, body.fqn.clone());
            for r in &reads {
                state
                    .lineage
                    .add_edge(LineageEdge::transforms(*r, id).with_kind("reads"));
            }
            for p in &produces {
                state
                    .lineage
                    .add_edge(LineageEdge::transforms(id, *p).with_kind("produces"));
            }
        }
    }

    let node = state.lineage.require_node(&id)?;
    Ok(Json(node))
}

async fn parse_sql(
    State(state): State<AppState>,
    Json(body): Json<ParseSqlRequest>,
) -> ApiResult<Json<SqlIngestResult>> {
    let target = match body.target_asset_id {
        Some(s) => Some(s.parse().map_err(ApiError::from)?),
        None => None,
    };
    Ok(Json(state.lineage.ingest_sql(&body.sql, target)?))
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

async fn column_upstream(
    State(state): State<AppState>,
    Path((id, column)): Path<(String, String)>,
    Query(q): Query<DepthQuery>,
) -> ApiResult<Json<ColumnLineageResponse>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let items = state.lineage.column_upstream(&asset_id, &column, q.depth);
    let count = items.len();
    Ok(Json(ColumnLineageResponse { items, count }))
}

async fn column_downstream(
    State(state): State<AppState>,
    Path((id, column)): Path<(String, String)>,
    Query(q): Query<DepthQuery>,
) -> ApiResult<Json<ColumnLineageResponse>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let items = state.lineage.column_downstream(&asset_id, &column, q.depth);
    let count = items.len();
    Ok(Json(ColumnLineageResponse { items, count }))
}

async fn table_impact(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DepthQuery>,
) -> ApiResult<Json<ImpactReport>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.lineage.impact_table_change(&asset_id, q.depth)))
}

async fn validation_impact(
    State(state): State<AppState>,
    Json(body): Json<ValidationImpactRequest>,
) -> ApiResult<Json<ImpactReport>> {
    let asset_id = body.asset_id.parse().map_err(ApiError::from)?;
    Ok(Json(state.lineage.impact_validation_failed(
        &asset_id,
        body.check_id,
        body.message,
        body.depth,
    )))
}
