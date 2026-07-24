//! Job scheduler routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use drp_core::{JobDefinition, JobRun};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    name: String,
    kind: String,
    #[serde(default)]
    schedule: Option<String>,
    #[serde(default)]
    params: indexmap::IndexMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct JobListResponse {
    items: Vec<JobDefinition>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct RunListResponse {
    items: Vec<JobRun>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/jobs", get(list_jobs).post(create_job))
        .route("/v1/jobs/{id}", get(get_job))
        .route("/v1/jobs/{id}/run", post(run_job))
        .route("/v1/jobs/{id}/runs", get(list_runs))
        .route("/v1/job-runs/{id}", get(get_run))
}

async fn list_jobs(State(state): State<AppState>) -> ApiResult<Json<JobListResponse>> {
    let items = state.scheduler.list_jobs().await?;
    let count = items.len();
    Ok(Json(JobListResponse { items, count }))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<JobDefinition>> {
    let job_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.scheduler.get_job(&job_id).await?))
}

async fn create_job(
    State(state): State<AppState>,
    Json(body): Json<CreateJobRequest>,
) -> ApiResult<Json<JobDefinition>> {
    let mut job = JobDefinition::new(body.name, body.kind);
    job.schedule = body.schedule;
    job.params = body.params;
    Ok(Json(state.scheduler.upsert_job(job).await?))
}

async fn run_job(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<JobRun>> {
    let job_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.scheduler.run_job(&job_id).await?))
}

async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ListRunsQuery>,
) -> ApiResult<Json<RunListResponse>> {
    let job_id = id.parse().map_err(ApiError::from)?;
    let items = state.scheduler.list_runs(&job_id, q.limit).await?;
    let count = items.len();
    Ok(Json(RunListResponse { items, count }))
}

async fn get_run(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<JobRun>> {
    let run_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.scheduler.get_run(&run_id).await?))
}
