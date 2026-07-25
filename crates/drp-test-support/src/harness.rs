//! In-process platform harness (no network listen required).

use std::sync::Arc;

use drp_ai::register_builtin_ai_providers;
use drp_anomaly::{register_builtin_detectors, AnomalyService};
use drp_api::{build_app, build_router, AppState};
use drp_common::{AppConfig, NotificationsConfig};
use drp_connectors::{
    register_builtin_connectors, FailingConnector, FixtureConnector, MockConnector,
};
use drp_core::{EventBus, PluginRegistry};
use drp_incidents::IncidentService;
use drp_lineage::LineageService;
use drp_metadata::MetadataService;
use drp_notifications::{register_builtin_notifiers, NotificationService};
use drp_profiling::{register_builtin_profilers, ProfilingService};
use drp_scheduler::SchedulerService;
use drp_storage::{MemoryStore, Store};
use drp_validation::{register_builtin_validators, ValidationService};

/// Fully wired platform services for tests.
pub struct TestPlatform {
    /// Shared app state (also used by HTTP router).
    pub state: AppState,
    /// Store handle.
    pub store: Arc<dyn Store>,
    /// Plugin registry.
    pub plugins: PluginRegistry,
}

impl TestPlatform {
    /// Build with defaults + built-in plugins (async).
    pub async fn new() -> Self {
        let mut config = AppConfig::default();
        config.logging.level = "error".into();
        config.logging.format = "pretty".into();
        config.scheduler.enabled = false;
        config.storage.backend = "memory".into();

        let state = build_app(config).await.expect("build_app for tests");
        let store = state.store.clone();
        let plugins = state.platform.plugins.clone();
        Self {
            state,
            store,
            plugins,
        }
    }

    /// Register an extra failing connector for negative tests.
    pub fn with_failing_connector(self) -> Self {
        self.plugins
            .register_connector(Arc::new(FailingConnector::new("test failure")));
        self
    }

    /// Axum router bound to this platform (for HTTP integration tests).
    pub fn router(&self) -> axum::Router {
        build_router(self.state.clone())
    }
}

/// Lower-level harness without HTTP / metrics init (pure service tests).
pub struct PlatformHarness {
    /// Store.
    pub store: Arc<dyn Store>,
    /// Plugins.
    pub plugins: PluginRegistry,
    /// Events.
    pub events: EventBus,
    /// Metadata.
    pub metadata: MetadataService,
    /// Profiling.
    pub profiling: ProfilingService,
    /// Validation.
    pub validation: ValidationService,
    /// Lineage.
    pub lineage: LineageService,
    /// Scheduler.
    pub scheduler: SchedulerService,
    /// Notifications.
    pub notifications: NotificationService,
    /// Incidents.
    pub incidents: IncidentService,
    /// Anomaly.
    pub anomaly: AnomalyService,
}

impl PlatformHarness {
    /// Compose services with built-in plugins on an in-memory store.
    pub fn new() -> Self {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let plugins = PluginRegistry::new();
        register_builtin_connectors(&plugins);
        register_builtin_profilers(&plugins);
        register_builtin_validators(&plugins);
        register_builtin_detectors(&plugins);
        register_builtin_notifiers(
            &plugins,
            &NotificationsConfig {
                enabled: true,
                default_channels: vec!["log".into()],
                slack_webhook_url: String::new(),
                email_to: String::new(),
                email_webhook_url: String::new(),
                webhook_url: String::new(),
            },
        );
        register_builtin_ai_providers(&plugins);

        plugins.register_connector(Arc::new(MockConnector::new()));
        plugins.register_connector(Arc::new(FixtureConnector::with_sample_data()));

        let events = EventBus::new();
        let metadata = MetadataService::new(store.clone(), plugins.clone(), events.clone());
        let profiling =
            ProfilingService::new(store.clone(), plugins.clone(), events.clone(), 10_000);
        let notifications = NotificationService::new(plugins.clone(), vec!["log".into()], true);
        let incidents =
            IncidentService::new(store.clone(), events.clone(), notifications.clone(), true);
        let validation = ValidationService::new(
            store.clone(),
            plugins.clone(),
            events.clone(),
            10_000,
            false,
        )
        .with_incidents(incidents.clone());
        let lineage = LineageService::new(20);
        let scheduler = SchedulerService::new(store.clone(), events.clone(), 4);
        let anomaly = AnomalyService::new(
            store.clone(),
            plugins.clone(),
            events.clone(),
            10_000,
            Default::default(),
            incidents.clone(),
        );

        Self {
            store,
            plugins,
            events,
            metadata,
            profiling,
            validation,
            lineage,
            scheduler,
            notifications,
            incidents,
            anomaly,
        }
    }
}

impl Default for PlatformHarness {
    fn default() -> Self {
        Self::new()
    }
}
