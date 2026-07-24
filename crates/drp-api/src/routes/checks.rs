//! Validation check routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use drp_common::{AssetId, Severity};
use drp_core::{CheckDefinition, CheckResult};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckRequest {
    name: String,
    asset_id: String,
    validator: String,
    #[serde(default)]
    severity: Option<Severity>,
    #[serde(default)]
    params: indexmap::IndexMap<String, Value>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunCheckRequest {
    #[serde(default = "default_connector")]
    connector: String,
}

fn default_connector() -> String {
    "mock".into()
}

#[derive(Debug, Serialize)]
pub struct CheckListResponse {
    items: Vec<CheckDefinition>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/checks", get(list_checks).post(create_check))
        .route("/v1/checks/{id}", get(get_check))
        .route("/v1/checks/{id}/run", post(run_check))
}

async fn list_checks(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<CheckListResponse>> {
    let asset_id = match q.asset_id {
        Some(s) => Some(s.parse::<AssetId>().map_err(ApiError::from)?),
        None => None,
    };
    let items = state.validation.list_checks(asset_id.as_ref()).await?;
    let count = items.len();
    Ok(Json(CheckListResponse { items, count }))
}

async fn get_check(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CheckDefinition>> {
    let check_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.validation.get_check(&check_id).await?))
}

async fn create_check(
    State(state): State<AppState>,
    Json(body): Json<CreateCheckRequest>,
) -> ApiResult<Json<CheckDefinition>> {
    let asset_id = body.asset_id.parse().map_err(ApiError::from)?;
    let mut check = CheckDefinition::new(body.name, asset_id, body.validator);
    if let Some(sev) = body.severity {
        check.severity = sev;
    }
    check.params = body.params;
    check.description = body.description;
    Ok(Json(state.validation.upsert_check(check).await?))
}

async fn run_check(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RunCheckRequest>,
) -> ApiResult<Json<CheckResult>> {
    let check_id = id.parse().map_err(ApiError::from)?;
    let result = state
        .validation
        .run_check(&check_id, &body.connector)
        .await?;

    if matches!(result.status, drp_common::ValidationStatus::Failed) {
        let mut meta = indexmap::IndexMap::new();
        meta.insert(
            "check_id".into(),
            serde_json::json!(result.check_id.to_string()),
        );
        let _ = state
            .notifications
            .notify("Data quality check failed", &result.message, meta)
            .await;
    }

    Ok(Json(result))
}
