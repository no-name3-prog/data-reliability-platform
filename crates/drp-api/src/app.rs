//! Application composition root.
//!
//! **Plugin registration happens only here** (or via thin `register_*` helpers
//! called from here). Feature services never import concrete plugin types.

use std::time::Duration;

use axum::http::{HeaderName, Request};
use axum::middleware;
use axum::Router;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};

use drp_ai::{register_ai_providers_with_config, AiService, RuleSuggestionService};
use drp_anomaly::{register_builtin_detectors, AnomalyService};
use drp_common::AppConfig;
use drp_connectors::register_builtin_connectors;
use drp_core::{init_tracing, Platform};
use drp_incidents::IncidentService;
use drp_lineage::LineageService;
use drp_metadata::MetadataService;
use drp_notifications::{register_builtin_notifiers, NotificationService};
use drp_profiling::{register_builtin_profilers, ProfilingService};
use drp_scheduler::SchedulerService;
use drp_storage::open_store;
use drp_validation::{register_builtin_validators, ValidationJobHandler, ValidationService};

use crate::metrics::{self, track_http_metrics};
use crate::routes;
use crate::state::AppState;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Register every built-in and workspace plugin bundle.
///
/// To add a third-party / new plugin crate: call its `register(&registry)` here.
/// Do not change `drp-core` or feature services.
fn register_all_plugins(platform: &Platform) {
    let reg = &platform.plugins;

    // Built-in capability crates
    register_builtin_connectors(reg);
    register_builtin_profilers(reg);
    register_builtin_validators(reg);
    register_builtin_detectors(reg);
    register_builtin_notifiers(reg, &platform.config.notifications);
    // AI providers registered in build_app after config is available
    // (see register_ai_providers_with_config).

    // Example external-style plugin (template for contributors)
    drp_plugin_example_connector::register(reg);
}

/// Build platform services and register built-in plugins.
pub async fn build_app(config: AppConfig) -> drp_common::Result<AppState> {
    init_tracing(&config)?;
    metrics::init_metrics()?;

    let platform = Platform::new(config);
    register_all_plugins(&platform);
    register_ai_providers_with_config(&platform.plugins, &platform.config.ai);

    info!(
        plugins = platform.plugins.len(),
        env = platform.environment(),
        ai_enabled = platform.config.ai.enabled,
        ai_default = %platform.config.ai.default_provider,
        postgres = %platform.config.infra.database_url,
        redis = %platform.config.infra.redis_url,
        s3 = %platform.config.infra.s3_endpoint,
        "platform plugins registered (infra URLs are compose DNS names)"
    );

    let store = open_store(&platform.config).await?;
    let events = platform.events.clone();
    let plugins = platform.plugins.clone();

    let metadata = MetadataService::new(store.clone(), plugins.clone(), events.clone());
    let profiling = ProfilingService::new(
        store.clone(),
        plugins.clone(),
        events.clone(),
        platform.config.profiling.sample_size,
    );
    let notifications = NotificationService::new(
        plugins.clone(),
        platform.config.notifications.default_channels.clone(),
        platform.config.notifications.enabled,
    );
    let incidents = IncidentService::new(
        store.clone(),
        events.clone(),
        notifications.clone(),
        platform.config.notifications.enabled,
    );
    let validation = ValidationService::new(
        store.clone(),
        plugins.clone(),
        events.clone(),
        platform.config.profiling.sample_size,
        platform.config.validation.fail_fast,
    )
    .with_incidents(incidents.clone());
    let lineage = LineageService::new(platform.config.lineage.max_depth);
    let scheduler = SchedulerService::new(
        store.clone(),
        events.clone(),
        platform.config.scheduler.max_concurrent_jobs,
    );
    // Register validation suite runner for scheduled jobs (kind = "validation").
    scheduler
        .handlers()
        .register(std::sync::Arc::new(ValidationJobHandler::new(
            validation.clone(),
        )));
    let anomaly = AnomalyService::new(
        store.clone(),
        plugins.clone(),
        events,
        platform.config.profiling.sample_size,
        platform.config.anomaly.clone(),
        incidents.clone(),
    );
    let ai = AiService::with_config(plugins.clone(), &platform.config.ai);
    let suggestions =
        RuleSuggestionService::new(store.clone(), plugins, platform.config.ai.clone());

    Ok(AppState {
        platform,
        store,
        metadata,
        profiling,
        validation,
        lineage,
        scheduler,
        notifications,
        incidents,
        anomaly,
        ai,
        suggestions,
    })
}

/// Build the Axum router with production middleware stack.
pub fn build_router(state: AppState) -> Router {
    let timeout = Duration::from_secs(state.platform.config.api.request_timeout_secs);

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request<_>| {
            let request_id = req
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-");
            tracing::info_span!(
                "http",
                method = %req.method(),
                uri = %req.uri().path(),
                request_id = %request_id,
            )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::ERROR));

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            REQUEST_ID_HEADER.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
        .layer(trace_layer)
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            timeout,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    Router::new()
        .merge(routes::router())
        .layer(middleware::from_fn(track_http_metrics))
        .layer(middleware)
        .with_state(state)
}
