//! Plugin introspection routes.

use axum::extract::State;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;

use drp_core::PluginInfo;

use crate::state::AppState;

#[derive(Serialize)]
struct PluginListResponse {
    items: Vec<PluginInfo>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/plugins", get(list_plugins))
}

async fn list_plugins(State(state): State<AppState>) -> Json<PluginListResponse> {
    let items = state.platform.plugins.list_all();
    let count = items.len();
    Json(PluginListResponse { items, count })
}
