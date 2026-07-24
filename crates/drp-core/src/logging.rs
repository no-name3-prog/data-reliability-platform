//! Structured tracing / logging initialization.
//!
//! Formats:
//! - `pretty` — human-readable (local containers)
//! - `json` — structured JSON for production log aggregators
//!
//! Fields always attached at process start via tracing spans in the API layer:
//! service, version, environment.

use drp_common::AppConfig;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the global tracing subscriber from config.
///
/// Safe to call once at process start. Subsequent calls are ignored.
pub fn init_tracing(cfg: &AppConfig) -> drp_common::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Include crate targets at configured level; hush noisy deps by default.
        EnvFilter::new(format!(
            "{},tower_http=info,hyper=warn,h2=warn",
            cfg.logging.level
        ))
    });

    let registry = tracing_subscriber::registry().with(filter);

    match cfg.logging.format.as_str() {
        "json" => {
            registry
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(true)
                        .with_span_list(true)
                        .with_target(true)
                        .with_thread_ids(false)
                        .with_file(false)
                        .with_line_number(false),
                )
                .try_init()
                .ok();
        }
        _ => {
            registry
                .with(
                    fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_thread_ids(false)
                        .with_file(false)
                        .with_line_number(false),
                )
                .try_init()
                .ok();
        }
    }

    tracing::info!(
        service = %cfg.platform.name,
        environment = %cfg.platform.environment,
        log_format = %cfg.logging.format,
        log_level = %cfg.logging.level,
        "structured logging initialized"
    );

    Ok(())
}
