//! In-process event bus for loose coupling between subsystems.

use std::sync::Arc;

use async_trait::async_trait;
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use drp_common::{AssetId, CheckId, JobId, RunId, ValidationStatus};

/// Platform-wide domain events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlatformEvent {
    /// An asset was registered or updated.
    AssetUpserted {
        /// Asset id.
        asset_id: AssetId,
    },
    /// A profile run finished.
    ProfileCompleted {
        /// Asset id.
        asset_id: AssetId,
        /// Run id.
        run_id: RunId,
    },
    /// A validation check finished.
    CheckCompleted {
        /// Check id.
        check_id: CheckId,
        /// Run id.
        run_id: RunId,
        /// Outcome.
        status: ValidationStatus,
    },
    /// A validation suite (batch) finished.
    ValidationRunCompleted {
        /// Suite run id.
        run_id: RunId,
        /// Optional asset scope.
        asset_id: Option<AssetId>,
        /// Aggregate status string (passed / warned / failed / error).
        status: String,
    },
    /// A job run finished.
    JobCompleted {
        /// Job id.
        job_id: JobId,
        /// Run id.
        run_id: RunId,
        /// Success flag.
        success: bool,
    },
    /// Generic extension point for plugins.
    Custom {
        /// Event name.
        name: String,
        /// Payload.
        payload: IndexMap<String, serde_json::Value>,
    },
}

/// Async listener for platform events.
#[async_trait]
pub trait EventListener: Send + Sync {
    /// Handle one event.
    async fn on_event(&self, event: &PlatformEvent);
}

type Listener = Arc<dyn EventListener>;

/// Simple fan-out event bus.
#[derive(Clone, Default)]
pub struct EventBus {
    listeners: Arc<RwLock<Vec<Listener>>>,
}

impl EventBus {
    /// Create an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a listener.
    pub fn subscribe(&self, listener: impl EventListener + 'static) {
        self.listeners.write().push(Arc::new(listener));
    }

    /// Publish an event to all listeners.
    pub async fn publish(&self, event: PlatformEvent) {
        debug!(?event, "publishing platform event");
        let listeners: Vec<Listener> = self.listeners.read().clone();
        for listener in listeners {
            if let Err(e) =
                tokio::time::timeout(std::time::Duration::from_secs(5), listener.on_event(&event))
                    .await
            {
                warn!(error = %e, "event listener timed out or failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CounterListener {
        n: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventListener for CounterListener {
        async fn on_event(&self, _: &PlatformEvent) {
            self.n.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn publishes_to_listeners() {
        let bus = EventBus::new();
        let n = Arc::new(AtomicUsize::new(0));
        bus.subscribe(CounterListener { n: n.clone() });
        bus.publish(PlatformEvent::AssetUpserted {
            asset_id: AssetId::new(),
        })
        .await;
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }
}
