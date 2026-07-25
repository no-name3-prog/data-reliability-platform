//! Data Reliability Platform — process entrypoint (container runtime).

use std::net::SocketAddr;
use std::time::Duration;

use clap::Parser;
use tracing::{info, warn};

use drp_api::{build_app, build_router};
use drp_common::{AppConfig, PRODUCT_NAME};

/// CLI arguments.
#[derive(Debug, Parser)]
#[command(
    name = "drp",
    about = "Data Reliability Platform server (container-first)",
    version
)]
struct Cli {
    /// Config directory (default: ./config).
    #[arg(long, env = "DRP_CONFIG_DIR", default_value = "config")]
    config_dir: String,

    /// Override bind host.
    #[arg(long, env = "DRP_API_HOST")]
    host: Option<String>,

    /// Override bind port.
    #[arg(long, env = "DRP_API_PORT")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let mut config = AppConfig::load_from(&cli.config_dir).unwrap_or_else(|e| {
        eprintln!("config load warning: {e}; using defaults");
        AppConfig::default()
    });

    if let Some(host) = cli.host {
        config.api.host = host;
    }
    if let Some(port) = cli.port {
        config.api.port = port;
    }

    let state = build_app(config).await?;
    let bind = state.platform.config.api.bind_addr();
    let scheduler_enabled = state.platform.config.scheduler.enabled;
    let tick = Duration::from_secs(state.platform.config.scheduler.tick_interval_secs);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    if scheduler_enabled {
        let sched = state.scheduler.clone();
        tokio::spawn(async move {
            sched.run_loop(tick, shutdown_rx).await;
        });
        info!("background scheduler enabled");
    }

    let app = build_router(state);
    let addr: SocketAddr = bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(
        %addr,
        product = PRODUCT_NAME,
        version = drp_api::VERSION,
        "listening (container-first runtime)"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    let _ = shutdown_tx.send(true);
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    warn!("shutdown signal received");
}
