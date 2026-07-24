//! HTTP route modules.

mod assets;
mod checks;
mod health;
mod jobs;
mod lineage;
mod plugins;
mod profiles;

use axum::routing::get;
use axum::Router;

use crate::metrics;
use crate::state::AppState;

/// Merge all route groups.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .route("/metrics", get(metrics::metrics_handler))
        .merge(assets::router())
        .merge(checks::router())
        .merge(profiles::router())
        .merge(lineage::router())
        .merge(jobs::router())
        .merge(plugins::router())
}
