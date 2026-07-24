//! In-process platform harness (no network listen required).

use std::sync::Arc;

use drp_api::{build_app, build_router, AppState};
use drp_common::AppConfig;
use drp_connectors::{
    register_builtin_connectors, FailingConnector, FixtureConnector, MockConnector,
};
use drp_core::{EventBus, PluginRegistry};
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
    /// Build with defaults + built-in plugins (mock, fixture, validators, …).
    pub fn new() -> Self {
        let mut config = AppConfig::default();
        config.logging.level = "error".into();
        config.logging.format = "pretty".into();
        config.scheduler.enabled = false;

        // build_app installs tracing/metrics — fine in tests (try_init).
        let state = build_app(config).expect("build_app for tests");
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

impl Default for TestPlatform {
    fn default() -> Self {
        Self::new()
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
}

impl PlatformHarness {
    /// Compose services with built-in plugins on an in-memory store.
    pub fn new() -> Self {
        let store: Arc<dyn Store> = Arc::new(MemoryStore::new());
        let plugins = PluginRegistry::new();
        register_builtin_connectors(&plugins);
        register_builtin_profilers(&plugins);
        register_builtin_validators(&plugins);
        register_builtin_notifiers(&plugins);

        // Ensure mock is present (also re-register for clarity).
        plugins.register_connector(Arc::new(MockConnector::new()));
        plugins.register_connector(Arc::new(FixtureConnector::with_sample_data()));

        let events = EventBus::new();
        let metadata = MetadataService::new(store.clone(), plugins.clone(), events.clone());
        let profiling =
            ProfilingService::new(store.clone(), plugins.clone(), events.clone(), 10_000);
        let validation = ValidationService::new(
            store.clone(),
            plugins.clone(),
            events.clone(),
            10_000,
            false,
        );
        let lineage = LineageService::new(20);
        let scheduler = SchedulerService::new(store.clone(), events.clone(), 4);
        let notifications = NotificationService::new(plugins.clone(), vec!["log".into()], true);

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
        }
    }
}

impl Default for PlatformHarness {
    fn default() -> Self {
        Self::new()
    }
}
