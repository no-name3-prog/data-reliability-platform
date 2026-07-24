//! AI provider routes.

use axum::extract::State;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;

use drp_core::{AiMessage, AiRequest, AiResponse, AiRole};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CompleteRequest {
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    prompt: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/ai/complete", post(complete))
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
