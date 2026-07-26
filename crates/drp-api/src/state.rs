//! Shared application state injected into handlers.

use std::sync::Arc;

use drp_ai::{AiService, RuleSuggestionService};
use drp_anomaly::AnomalyService;
use drp_core::Platform;
use drp_incidents::IncidentService;
use drp_lineage::LineageService;
use drp_metadata::MetadataService;
use drp_notifications::NotificationService;
use drp_profiling::ProfilingService;
use drp_scheduler::SchedulerService;
use drp_storage::Store;
use drp_validation::ValidationService;

/// Axum state.
#[derive(Clone)]
pub struct AppState {
    /// Core platform.
    pub platform: Platform,
    /// Persistence.
    pub store: Arc<dyn Store>,
    /// Metadata catalog.
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
    /// Incident management.
    pub incidents: IncidentService,
    /// Anomaly detection.
    pub anomaly: AnomalyService,
    /// AI completions.
    pub ai: AiService,
    /// AI validation rule suggestions (human-in-the-loop).
    pub suggestions: RuleSuggestionService,
}
