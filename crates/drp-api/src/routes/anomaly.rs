//! Anomaly detection routes.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;

use drp_core::AnomalyReport;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DetectRequest {
    #[serde(default = "default_connector")]
    connector: String,
    detector: Option<String>,
}

fn default_connector() -> String {
    "mock".into()
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/assets/{id}/anomalies/detect", post(detect))
}

async fn detect(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<DetectRequest>,
) -> ApiResult<Json<AnomalyReport>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let report = state
        .anomaly
        .detect(&asset_id, &body.connector, body.detector.as_deref())
        .await?;
    Ok(Json(report))
}
