//! Incident management routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use drp_common::AssetId;
use drp_core::{Incident, IncidentStatus, IncidentTimelineEvent};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<usize>,
    asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    status: IncidentStatus,
    #[serde(default)]
    actor: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignOwnerRequest {
    owner: String,
    #[serde(default)]
    actor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NoteRequest {
    message: String,
    #[serde(default)]
    actor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AffectedAssetsRequest {
    /// Asset ids (strings).
    assets: Vec<String>,
    #[serde(default)]
    actor: Option<String>,
    /// When true, also append lineage downstream assets.
    #[serde(default)]
    include_lineage_downstream: bool,
    #[serde(default)]
    depth: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct IncidentListResponse {
    items: Vec<Incident>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    items: Vec<IncidentTimelineEvent>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/incidents", get(list_incidents))
        .route("/v1/incidents/{id}", get(get_incident))
        .route("/v1/incidents/{id}/status", post(update_status))
        .route("/v1/incidents/{id}/owner", post(assign_owner))
        .route("/v1/incidents/{id}/notes", post(add_note))
        .route("/v1/incidents/{id}/history", get(history))
        .route("/v1/incidents/{id}/affected-assets", post(set_affected))
        // Keep PATCH-style alias used by anomaly routes previously
        .route("/v1/incidents/{id}", axum::routing::put(update_status_put))
}

async fn list_incidents(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<IncidentListResponse>> {
    let asset_id = match q.asset_id {
        Some(s) => Some(s.parse::<AssetId>().map_err(ApiError::from)?),
        None => None,
    };
    let items = state.incidents.list(asset_id.as_ref(), q.limit).await?;
    let count = items.len();
    Ok(Json(IncidentListResponse { items, count }))
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.incidents.get(&id).await?))
}

async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusRequest>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state
            .incidents
            .set_status(&id, body.status, body.actor, body.note)
            .await?,
    ))
}

async fn update_status_put(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusRequest>,
) -> ApiResult<Json<Incident>> {
    update_status(State(state), Path(id), Json(body)).await
}

async fn assign_owner(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AssignOwnerRequest>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state
            .incidents
            .assign_owner(&id, body.owner, body.actor)
            .await?,
    ))
}

async fn add_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NoteRequest>,
) -> ApiResult<Json<IncidentTimelineEvent>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state
            .incidents
            .add_note(&id, body.message, body.actor)
            .await?,
    ))
}

async fn history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<TimelineResponse>> {
    let id = id.parse().map_err(ApiError::from)?;
    let items = state.incidents.history(&id, q.limit).await?;
    let count = items.len();
    Ok(Json(TimelineResponse { items, count }))
}

async fn set_affected(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AffectedAssetsRequest>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    let mut assets: Vec<AssetId> = Vec::new();
    for s in body.assets {
        assets.push(s.parse().map_err(ApiError::from)?);
    }
    if body.include_lineage_downstream {
        let incident = state.incidents.get(&id).await?;
        let down = state.lineage.downstream(&incident.asset_id, body.depth);
        for n in down.nodes {
            if !assets.contains(&n.asset_id) {
                assets.push(n.asset_id);
            }
        }
    }
    Ok(Json(
        state
            .incidents
            .set_affected_assets(&id, assets, body.actor)
            .await?,
    ))
}
