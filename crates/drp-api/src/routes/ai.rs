//! AI provider and validation rule suggestion routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use drp_ai::ApproveResult;
use drp_common::{AssetId, SuggestionId};
use drp_core::{
    AiMessage, AiRequest, AiResponse, AiRole, PluginInfo, RuleSuggestion, RuleSuggestionStatus,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CompleteRequest {
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    prompt: String,
}

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    /// Connector used to sample rows (default mock).
    #[serde(default = "default_connector")]
    connector: String,
    /// AI provider plugin id (default from config, usually `heuristic`).
    #[serde(default)]
    provider: Option<String>,
}

fn default_connector() -> String {
    "mock".into()
}

#[derive(Debug, Deserialize)]
pub struct ListSuggestionsQuery {
    asset_id: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    #[serde(default)]
    reviewed_by: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    items: Vec<T>,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct AiStatusResponse {
    enabled: bool,
    default_provider: String,
    providers: Vec<PluginInfo>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ai/complete", post(complete))
        .route("/v1/ai/providers", get(list_providers))
        .route("/v1/ai/status", get(ai_status))
        .route("/v1/assets/{id}/ai/suggest-rules", post(suggest_rules))
        .route("/v1/ai/suggestions", get(list_suggestions))
        .route("/v1/ai/suggestions/{id}", get(get_suggestion))
        .route("/v1/ai/suggestions/{id}/approve", post(approve_suggestion))
        .route("/v1/ai/suggestions/{id}/reject", post(reject_suggestion))
}

async fn complete(
    State(state): State<AppState>,
    Json(body): Json<CompleteRequest>,
) -> ApiResult<Json<AiResponse>> {
    let request = AiRequest {
        model: body.model,
        messages: vec![
            AiMessage {
                role: AiRole::System,
                content: "You are a data reliability assistant.".into(),
            },
            AiMessage::user(body.prompt),
        ],
        temperature: None,
        max_tokens: None,
        options: indexmap::IndexMap::new(),
    };
    let resp = state.ai.complete(request, body.provider.as_deref()).await?;
    Ok(Json(resp))
}

async fn list_providers(
    State(state): State<AppState>,
) -> ApiResult<Json<ListResponse<PluginInfo>>> {
    let items = state.suggestions.list_providers();
    let count = items.len();
    Ok(Json(ListResponse { items, count }))
}

async fn ai_status(State(state): State<AppState>) -> ApiResult<Json<AiStatusResponse>> {
    Ok(Json(AiStatusResponse {
        enabled: state.suggestions.enabled(),
        default_provider: state.suggestions.default_provider().to_string(),
        providers: state.suggestions.list_providers(),
    }))
}

async fn suggest_rules(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SuggestRequest>,
) -> ApiResult<Json<ListResponse<RuleSuggestion>>> {
    let asset_id: AssetId = id
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid asset id: {e}")))?;
    let items = state
        .suggestions
        .suggest_for_asset(&asset_id, &body.connector, body.provider.as_deref())
        .await?;
    let count = items.len();
    Ok(Json(ListResponse { items, count }))
}

async fn list_suggestions(
    State(state): State<AppState>,
    Query(q): Query<ListSuggestionsQuery>,
) -> ApiResult<Json<ListResponse<RuleSuggestion>>> {
    let asset_id = match q.asset_id {
        Some(s) => Some(
            s.parse::<AssetId>()
                .map_err(|e| ApiError::bad_request(format!("invalid asset_id: {e}")))?,
        ),
        None => None,
    };
    let status = match q.status.as_deref() {
        None | Some("") | Some("all") => None,
        Some("pending") => Some(RuleSuggestionStatus::Pending),
        Some("approved") => Some(RuleSuggestionStatus::Approved),
        Some("rejected") => Some(RuleSuggestionStatus::Rejected),
        Some(other) => {
            return Err(ApiError::bad_request(format!(
                "invalid status '{other}' (use pending|approved|rejected)"
            )));
        }
    };
    let items = state
        .suggestions
        .list(asset_id.as_ref(), status, q.limit)
        .await?;
    let count = items.len();
    Ok(Json(ListResponse { items, count }))
}

async fn get_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RuleSuggestion>> {
    let sid: SuggestionId = id
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid suggestion id: {e}")))?;
    let s = state.suggestions.get(&sid).await?;
    Ok(Json(s))
}

async fn approve_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ReviewRequest>>,
) -> ApiResult<Json<ApproveResult>> {
    let sid: SuggestionId = id
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid suggestion id: {e}")))?;
    let reviewed_by = body.and_then(|b| b.0.reviewed_by);
    let result = state.suggestions.approve(&sid, reviewed_by).await?;
    Ok(Json(result))
}

async fn reject_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<ReviewRequest>>,
) -> ApiResult<Json<RuleSuggestion>> {
    let sid: SuggestionId = id
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid suggestion id: {e}")))?;
    let (reason, reviewed_by) = match body {
        Some(Json(b)) => (b.reason, b.reviewed_by),
        None => (None, None),
    };
    let s = state.suggestions.reject(&sid, reason, reviewed_by).await?;
    Ok(Json(s))
}
