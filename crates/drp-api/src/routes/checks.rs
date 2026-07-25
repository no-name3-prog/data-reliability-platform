//! Validation check and suite routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use drp_common::{AssetId, CheckId, Severity};
use drp_core::{CheckDefinition, CheckResult, JobDefinition, PluginInfo, ValidationRun};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    limit: Option<usize>,
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
    /// Optional schedule expression; creates a linked `validation` job.
    #[serde(default)]
    schedule: Option<String>,
    /// Connector used when creating a scheduled job (default mock).
    #[serde(default = "default_connector")]
    connector: String,
}

#[derive(Debug, Deserialize)]
pub struct RunCheckRequest {
    #[serde(default = "default_connector")]
    connector: String,
}

#[derive(Debug, Deserialize)]
pub struct RunSuiteRequest {
    #[serde(default = "default_connector")]
    connector: String,
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    check_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleRequest {
    name: String,
    asset_id: String,
    schedule: String,
    #[serde(default = "default_connector")]
    connector: String,
    #[serde(default)]
    check_ids: Option<Vec<String>>,
}

fn default_connector() -> String {
    "mock".into()
}

#[derive(Debug, Serialize)]
pub struct CheckListResponse {
    items: Vec<CheckDefinition>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct ResultListResponse {
    items: Vec<CheckResult>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct SuiteListResponse {
    items: Vec<ValidationRun>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct RulesListResponse {
    items: Vec<PluginInfo>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/checks", get(list_checks).post(create_check))
        .route("/v1/checks/{id}", get(get_check))
        .route("/v1/checks/{id}/run", post(run_check))
        .route("/v1/checks/{id}/results", get(list_check_results))
        .route("/v1/validation/rules", get(list_rules))
        .route("/v1/validation/runs", get(list_suite_runs).post(run_suite))
        .route("/v1/validation/runs/{id}", get(get_suite_run))
        .route("/v1/validation/schedule", post(schedule_validation))
        .route("/v1/assets/{id}/validate", post(validate_asset))
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
    check.schedule = body.schedule;
    let saved = if check.schedule.is_some() {
        state
            .validation
            .upsert_check_with_schedule(check, &body.connector)
            .await?
    } else {
        state.validation.upsert_check(check).await?
    };
    Ok(Json(saved))
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

async fn list_check_results(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> ApiResult<Json<ResultListResponse>> {
    let check_id: CheckId = id.parse().map_err(ApiError::from)?;
    let items = state
        .validation
        .list_check_results(&check_id, q.limit)
        .await?;
    let count = items.len();
    Ok(Json(ResultListResponse { items, count }))
}

async fn list_rules(State(state): State<AppState>) -> ApiResult<Json<RulesListResponse>> {
    let items = state.validation.list_rules();
    let count = items.len();
    Ok(Json(RulesListResponse { items, count }))
}

async fn run_suite(
    State(state): State<AppState>,
    Json(body): Json<RunSuiteRequest>,
) -> ApiResult<Json<ValidationRun>> {
    let asset_id = match body.asset_id {
        Some(s) => Some(s.parse::<AssetId>().map_err(ApiError::from)?),
        None => None,
    };
    let check_ids = match body.check_ids {
        Some(ids) => {
            let mut out = Vec::new();
            for id in ids {
                out.push(id.parse::<CheckId>().map_err(ApiError::from)?);
            }
            Some(out)
        }
        None => None,
    };
    Ok(Json(
        state
            .validation
            .run_suite(asset_id, &body.connector, check_ids, None)
            .await?,
    ))
}

async fn list_suite_runs(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> ApiResult<Json<SuiteListResponse>> {
    let asset_id = match q.asset_id {
        Some(s) => Some(s.parse::<AssetId>().map_err(ApiError::from)?),
        None => None,
    };
    let items = state
        .validation
        .list_validation_runs(asset_id.as_ref(), q.limit)
        .await?;
    let count = items.len();
    Ok(Json(SuiteListResponse { items, count }))
}

async fn get_suite_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ValidationRun>> {
    let run_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.validation.get_validation_run(&run_id).await?))
}

async fn validate_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RunCheckRequest>,
) -> ApiResult<Json<ValidationRun>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(
        state
            .validation
            .run_checks_for_asset(&asset_id, &body.connector)
            .await?,
    ))
}

async fn schedule_validation(
    State(state): State<AppState>,
    Json(body): Json<ScheduleRequest>,
) -> ApiResult<Json<JobDefinition>> {
    let asset_id = body.asset_id.parse().map_err(ApiError::from)?;
    let check_ids = match body.check_ids {
        Some(ids) => {
            let mut out = Vec::new();
            for id in ids {
                out.push(id.parse::<CheckId>().map_err(ApiError::from)?);
            }
            Some(out)
        }
        None => None,
    };
    Ok(Json(
        state
            .validation
            .schedule_asset_validation(
                body.name,
                asset_id,
                body.connector,
                body.schedule,
                check_ids,
            )
            .await?,
    ))
}
