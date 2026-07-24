//! Application composition root.

use std::time::Duration;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use drp_common::AppConfig;
use drp_connectors::register_builtin_connectors;
use drp_core::{init_tracing, Platform};
use drp_lineage::LineageService;
use drp_metadata::MetadataService;
use drp_notifications::{register_builtin_notifiers, NotificationService};
use drp_profiling::{register_builtin_profilers, ProfilingService};
use drp_scheduler::SchedulerService;
use drp_storage::open_store;
use drp_validation::{register_builtin_validators, ValidationService};

use crate::routes;
use crate::state::AppState;

/// Build platform services and register built-in plugins.
pub fn build_app(config: AppConfig) -> drp_common::Result<AppState> {
    init_tracing(&config)?;

    let platform = Platform::new(config);
    register_builtin_connectors(&platform.plugins);
    register_builtin_profilers(&platform.plugins);
    register_builtin_validators(&platform.plugins);
    register_builtin_notifiers(&platform.plugins);

    info!(
        plugins = platform.plugins.len(),
        env = platform.environment(),
        postgres = %platform.config.infra.database_url,
        redis = %platform.config.infra.redis_url,
        s3 = %platform.config.infra.s3_endpoint,
        "platform plugins registered (infra URLs are compose DNS names)"
    );

    let store = open_store(&platform.config)?;
    let events = platform.events.clone();
    let plugins = platform.plugins.clone();

    let metadata = MetadataService::new(store.clone(), plugins.clone(), events.clone());
    let profiling = ProfilingService::new(
        store.clone(),
        plugins.clone(),
        events.clone(),
        platform.config.profiling.sample_size,
    );
    let validation = ValidationService::new(
        store.clone(),
        plugins.clone(),
        events.clone(),
        platform.config.profiling.sample_size,
        platform.config.validation.fail_fast,
    );
    let lineage = LineageService::new(platform.config.lineage.max_depth);
    let scheduler = SchedulerService::new(
        store.clone(),
        events,
        platform.config.scheduler.max_concurrent_jobs,
    );
    let notifications = NotificationService::new(
        plugins,
        platform.config.notifications.default_channels.clone(),
        platform.config.notifications.enabled,
    );

    Ok(AppState {
        platform,
        store,
        metadata,
        profiling,
        validation,
        lineage,
        scheduler,
        notifications,
    })
}

/// Build the Axum router with middleware.
pub fn build_router(state: AppState) -> Router {
    let timeout = Duration::from_secs(state.platform.config.api.request_timeout_secs);

    Router::new()
        .merge(routes::router())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            timeout,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}
