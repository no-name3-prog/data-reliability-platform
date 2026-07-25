//! Configuration loading (layered files + env). Environment variables use `DRP_` prefix.

use serde::{Deserialize, Serialize};

/// Error raised while loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Underlying config crate failure.
    #[error("failed to load config: {0}")]
    Load(String),
    /// Deserialization / schema failure.
    #[error("invalid config: {0}")]
    Invalid(String),
}

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Platform identity.
    pub platform: PlatformConfig,
    /// Logging.
    pub logging: LoggingConfig,
    /// HTTP API.
    pub api: ApiConfig,
    /// Persistence.
    pub storage: StorageConfig,
    /// Job scheduler.
    pub scheduler: SchedulerConfig,
    /// Notifications.
    pub notifications: NotificationsConfig,
    /// Profiling defaults.
    pub profiling: ProfilingConfig,
    /// Validation defaults.
    pub validation: ValidationConfig,
    /// Lineage defaults.
    pub lineage: LineageConfig,
    /// Anomaly detection defaults.
    #[serde(default)]
    pub anomaly: AnomalyConfig,
    /// Infra connection strings (provided by compose in containers).
    pub infra: InfraConfig,
}

/// Platform identity section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// Service name.
    pub name: String,
    /// Environment label.
    pub environment: String,
}

/// Logging section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default filter level.
    pub level: String,
    /// `pretty` or `json`.
    pub format: String,
}

/// HTTP API section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Bind host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Allowed CORS origins.
    pub cors_allow_origins: Vec<String>,
}

impl ApiConfig {
    /// Socket address string `host:port`.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Storage section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend id: `memory` | `postgres` (postgres backend pluggable later).
    pub backend: String,
    /// Connection string when applicable.
    pub database_url: String,
    /// Pool size hint.
    pub max_connections: u32,
}

/// Scheduler section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Whether the in-process scheduler runs.
    pub enabled: bool,
    /// Poll interval in seconds.
    pub tick_interval_secs: u64,
    /// Cap on concurrent job executions.
    pub max_concurrent_jobs: usize,
}

/// Notifications section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    /// Master switch.
    pub enabled: bool,
    /// Default channel plugin ids (e.g. log, slack, email, webhook).
    pub default_channels: Vec<String>,
    /// Slack incoming webhook URL (empty = dry-run log).
    #[serde(default)]
    pub slack_webhook_url: String,
    /// Email recipient (empty = dry-run log).
    #[serde(default)]
    pub email_to: String,
    /// Optional SMTP-style HTTP email bridge URL (SendGrid-like webhook).
    #[serde(default)]
    pub email_webhook_url: String,
    /// Generic webhook URL for incident payloads (empty = dry-run log).
    #[serde(default)]
    pub webhook_url: String,
}

/// Profiling defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Default sample size.
    pub sample_size: usize,
    /// Null-ratio threshold for warnings.
    pub null_threshold: f64,
}

/// Validation defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Stop on first hard failure.
    pub fail_fast: bool,
    /// Default severity string.
    pub default_severity: String,
}

/// Lineage defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageConfig {
    /// Maximum traversal depth.
    pub max_depth: u32,
}

/// Anomaly / profile-drift defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// How many historical profiles (excluding latest) to consider.
    pub history_window: usize,
    /// Flag when row_count drops by at least this fraction of baseline (e.g. 0.3 = 30%).
    pub row_count_drop_ratio: f64,
    /// Flag when null_percentage rises by at least this many points vs baseline.
    pub null_spike_delta: f64,
    /// Flag when unique_ratio drops by at least this absolute amount (0–1 scale).
    pub duplicate_unique_ratio_drop: f64,
    /// Flag distribution when |mean delta| / max(stddev, eps) exceeds this z-like threshold.
    pub distribution_zscore: f64,
    /// Max age in seconds of the latest profile before freshness incidents fire.
    pub freshness_max_age_secs: u64,
    /// When true, open an incident for each finding.
    pub create_incidents: bool,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            history_window: 10,
            row_count_drop_ratio: 0.3,
            null_spike_delta: 10.0,
            duplicate_unique_ratio_drop: 0.2,
            distribution_zscore: 3.0,
            freshness_max_age_secs: 86_400,
            create_incidents: true,
        }
    }
}

/// External infrastructure URLs (compose service DNS names in containers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraConfig {
    /// Postgres URL.
    pub database_url: String,
    /// Redis URL.
    pub redis_url: String,
    /// S3-compatible endpoint (MinIO).
    pub s3_endpoint: String,
    /// S3 access key.
    pub s3_access_key: String,
    /// S3 secret key.
    pub s3_secret_key: String,
    /// Default bucket.
    pub s3_bucket: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            platform: PlatformConfig {
                name: "data-reliability-platform".into(),
                environment: "development".into(),
            },
            logging: LoggingConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
            api: ApiConfig {
                host: "0.0.0.0".into(),
                port: 8080,
                request_timeout_secs: 30,
                cors_allow_origins: vec!["*".into()],
            },
            storage: StorageConfig {
                backend: "memory".into(),
                database_url: "postgres://drp:drp@postgres:5432/drp".into(),
                max_connections: 10,
            },
            scheduler: SchedulerConfig {
                enabled: true,
                tick_interval_secs: 5,
                max_concurrent_jobs: 8,
            },
            notifications: NotificationsConfig {
                enabled: true,
                default_channels: vec![
                    "log".into(),
                    "slack".into(),
                    "email".into(),
                    "webhook".into(),
                ],
                slack_webhook_url: String::new(),
                email_to: String::new(),
                email_webhook_url: String::new(),
                webhook_url: String::new(),
            },
            profiling: ProfilingConfig {
                sample_size: 10_000,
                null_threshold: 0.05,
            },
            validation: ValidationConfig {
                fail_fast: false,
                default_severity: "error".into(),
            },
            lineage: LineageConfig { max_depth: 20 },
            anomaly: AnomalyConfig::default(),
            infra: InfraConfig {
                database_url: "postgres://drp:drp@postgres:5432/drp".into(),
                redis_url: "redis://redis:6379".into(),
                s3_endpoint: "http://minio:9000".into(),
                s3_access_key: "minioadmin".into(),
                s3_secret_key: "minioadmin".into(),
                s3_bucket: "drp".into(),
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from default paths and `DRP_*` environment variables.
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from("config")
    }

    /// Load from a config directory root.
    pub fn load_from(config_dir: &str) -> Result<Self, ConfigError> {
        let env = std::env::var("DRP_ENV").unwrap_or_else(|_| "development".into());

        let builder = config::Config::builder()
            .add_source(config::File::with_name(&format!("{config_dir}/default")).required(false))
            .add_source(config::File::with_name(&format!("{config_dir}/{env}")).required(false))
            .add_source(
                config::Environment::with_prefix("DRP")
                    .separator("__")
                    .try_parsing(true),
            );

        let cfg = builder
            .build()
            .map_err(|e| ConfigError::Load(e.to_string()))?;

        let mut merged = AppConfig::default();
        if let Ok(partial) = cfg.try_deserialize::<serde_json::Value>() {
            let defaults =
                serde_json::to_value(&merged).map_err(|e| ConfigError::Invalid(e.to_string()))?;
            let merged_value = merge_json(defaults, partial);
            merged = serde_json::from_value(merged_value)
                .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        }

        apply_env_overrides(&mut merged);
        Ok(merged)
    }
}

fn apply_env_overrides(merged: &mut AppConfig) {
    if let Ok(port) = std::env::var("DRP_API_PORT") {
        if let Ok(p) = port.parse() {
            merged.api.port = p;
        }
    }
    if let Ok(host) = std::env::var("DRP_API_HOST") {
        merged.api.host = host;
    }
    if let Ok(level) = std::env::var("DRP_LOG_LEVEL") {
        merged.logging.level = level;
    }
    if let Ok(format) = std::env::var("DRP_LOG_FORMAT") {
        merged.logging.format = format;
    }
    if let Ok(backend) = std::env::var("DRP_STORAGE_BACKEND") {
        merged.storage.backend = backend;
    }
    if let Ok(url) = std::env::var("DRP_DATABASE_URL") {
        merged.storage.database_url = url.clone();
        merged.infra.database_url = url;
    }
    if let Ok(url) = std::env::var("DRP_REDIS_URL") {
        merged.infra.redis_url = url;
    }
    if let Ok(url) = std::env::var("DRP_S3_ENDPOINT") {
        merged.infra.s3_endpoint = url;
    }
    if let Ok(v) = std::env::var("DRP_S3_ACCESS_KEY") {
        merged.infra.s3_access_key = v;
    }
    if let Ok(v) = std::env::var("DRP_S3_SECRET_KEY") {
        merged.infra.s3_secret_key = v;
    }
    if let Ok(v) = std::env::var("DRP_S3_BUCKET") {
        merged.infra.s3_bucket = v;
    }
    if let Ok(env) = std::env::var("DRP_ENV") {
        merged.platform.environment = env;
    }
}

fn merge_json(mut base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    match (&mut base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (k, v) in overlay_map {
                let entry = base_map.entry(k).or_insert(serde_json::Value::Null);
                *entry = merge_json(entry.take(), v);
            }
            base
        }
        (_, overlay) => overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.api.port, 8080);
        assert_eq!(cfg.storage.backend, "memory");
        assert!(cfg.infra.database_url.contains("postgres"));
    }
}
