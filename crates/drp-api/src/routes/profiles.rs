//! Profiling routes.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;

use drp_core::DatasetProfile;

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

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/v1/assets/{id}/profile",
        post(run_profile).get(get_profile),
    )
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
