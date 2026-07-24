//! HTTP route modules.

mod assets;
mod checks;
mod health;
mod jobs;
mod lineage;
mod plugins;
mod profiles;

use axum::Router;

use crate::state::AppState;

/// Merge all route groups.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(assets::router())
        .merge(checks::router())
        .merge(profiles::router())
        .merge(lineage::router())
        .merge(jobs::router())
        .merge(plugins::router())
}
