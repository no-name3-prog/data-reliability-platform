//! Profiling routes — run, latest, history, and compare over time.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::Deserialize;

use drp_core::{DatasetProfile, ProfileDiff};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ProfileRequest {
    #[serde(default = "default_connector")]
    connector: String,
    profiler: Option<String>,
}

fn default_connector() -> String {
    "mock".into()
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    /// Baseline run id (default: previous run).
    baseline: Option<String>,
    /// Current run id (default: latest).
    current: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/assets/{id}/profile",
            post(run_profile).get(get_profile),
        )
        .route("/v1/assets/{id}/profiles", get(list_history))
        .route("/v1/assets/{id}/profiles/compare", get(compare))
        .route("/v1/assets/{id}/profiles/{run_id}", get(get_run))
}

async fn run_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ProfileRequest>,
) -> ApiResult<Json<DatasetProfile>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let profile = state
        .profiling
        .profile_asset(&asset_id, &body.connector, body.profiler.as_deref())
        .await?;
    Ok(Json(profile))
}

async fn get_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Option<DatasetProfile>>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.profiling.latest_profile(&asset_id).await?))
}

async fn list_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let items = state.profiling.profile_history(&asset_id, q.limit).await?;
    let count = items.len();
    Ok(Json(serde_json::json!({ "items": items, "count": count })))
}

async fn get_run(
    State(state): State<AppState>,
    Path((id, run_id)): Path<(String, String)>,
) -> ApiResult<Json<Option<DatasetProfile>>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let run_id = run_id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state.profiling.get_profile_run(&asset_id, &run_id).await?,
    ))
}

async fn compare(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<CompareQuery>,
) -> ApiResult<Json<ProfileDiff>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let baseline = q
        .baseline
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(ApiError::from)?;
    let current = q
        .current
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(ApiError::from)?;
    let diff = state
        .profiling
        .compare_profiles(&asset_id, baseline.as_ref(), current.as_ref())
        .await?;
    Ok(Json(diff))
}
