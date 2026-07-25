//! Anomaly detection routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
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

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReportListResponse {
    items: Vec<AnomalyReport>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/assets/{id}/anomalies/analyze", post(analyze_profiles))
        .route("/v1/assets/{id}/anomalies/detect", post(detect))
        .route("/v1/assets/{id}/anomalies/reports", get(list_reports))
        .route("/v1/anomaly-reports/{run_id}", get(get_report))
}

async fn analyze_profiles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AnomalyReport>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let report = state.anomaly.analyze_profiles(&asset_id).await?;
    Ok(Json(report))
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

async fn list_reports(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ReportListResponse>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let items = state.anomaly.list_reports(&asset_id, q.limit).await?;
    let count = items.len();
    Ok(Json(ReportListResponse { items, count }))
}

async fn get_report(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<AnomalyReport>> {
    let run_id = run_id.parse().map_err(ApiError::from)?;
    Ok(Json(state.anomaly.get_report(&run_id).await?))
}
