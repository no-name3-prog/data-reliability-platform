//! Anomaly detection and incident routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use drp_common::AssetId;
use drp_core::{AnomalyReport, Incident, IncidentStatus};

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
    asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    status: IncidentStatus,
}

#[derive(Debug, Serialize)]
pub struct ReportListResponse {
    items: Vec<AnomalyReport>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct IncidentListResponse {
    items: Vec<Incident>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Profile-history analysis (primary engine)
        .route("/v1/assets/{id}/anomalies/analyze", post(analyze_profiles))
        // Sample-based detector plugins
        .route("/v1/assets/{id}/anomalies/detect", post(detect))
        .route("/v1/assets/{id}/anomalies/reports", get(list_reports))
        .route("/v1/anomaly-reports/{run_id}", get(get_report))
        .route("/v1/incidents", get(list_incidents))
        .route("/v1/incidents/{id}", get(get_incident).patch(update_incident))
}

async fn analyze_profiles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AnomalyReport>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    let report = state.anomaly.analyze_profiles(&asset_id).await?;
    maybe_notify_findings(&state, &report).await;
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
    maybe_notify_findings(&state, &report).await;
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

async fn list_incidents(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<IncidentListResponse>> {
    let asset_id = match q.asset_id {
        Some(s) => Some(s.parse::<AssetId>().map_err(ApiError::from)?),
        None => None,
    };
    let items = state
        .anomaly
        .list_incidents(asset_id.as_ref(), q.limit)
        .await?;
    let count = items.len();
    Ok(Json(IncidentListResponse { items, count }))
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.anomaly.get_incident(&id).await?))
}

async fn update_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateIncidentRequest>,
) -> ApiResult<Json<Incident>> {
    let id = id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state.anomaly.set_incident_status(&id, body.status).await?,
    ))
}

async fn maybe_notify_findings(state: &AppState, report: &AnomalyReport) {
    if !report.has_findings() {
        return;
    }
    let mut meta = indexmap::IndexMap::new();
    meta.insert(
        "asset_id".into(),
        serde_json::json!(report.asset_id.to_string()),
    );
    meta.insert(
        "run_id".into(),
        serde_json::json!(report.run_id.to_string()),
    );
    meta.insert(
        "finding_count".into(),
        serde_json::json!(report.findings.len()),
    );
    let subject = format!(
        "Anomaly report: {} finding(s) on asset {}",
        report.findings.len(),
        report.asset_id
    );
    let body = report
        .findings
        .iter()
        .map(|f| format!("- [{}] {}", f.kind.as_str(), f.message))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = state.notifications.notify(&subject, &body, meta).await;
}
