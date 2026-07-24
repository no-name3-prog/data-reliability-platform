//! Tracing / logging initialization.

use drp_common::AppConfig;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the global tracing subscriber from config.
pub fn init_tracing(cfg: &AppConfig) -> drp_common::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cfg.logging.level));

    let registry = tracing_subscriber::registry().with(filter);

    match cfg.logging.format.as_str() {
        "json" => {
            registry
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(true)
                        .with_span_list(true),
                )
                .try_init()
                .ok();
        }
        _ => {
            registry.with(fmt::layer().pretty()).try_init().ok();
        }
    }
    Ok(())
}
