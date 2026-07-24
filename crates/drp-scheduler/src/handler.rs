//! Job handler plugin interface.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::Value;
use tracing::info;

use drp_common::{Error, Result};
use drp_core::JobDefinition;

/// Executes a job of a specific kind.
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// Kind this handler supports.
    fn kind(&self) -> &str;
    /// Execute the job.
    async fn execute(&self, job: &JobDefinition) -> Result<Option<Value>>;
}

/// Registry of job handlers by kind.
#[derive(Clone, Default)]
pub struct JobHandlerRegistry {
    handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
}

impl JobHandlerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler.
    pub fn register(&self, handler: Arc<dyn JobHandler>) {
        let kind = handler.kind().to_string();
        info!(job_kind = %kind, "registering job handler");
        self.handlers.write().insert(kind, handler);
    }

    /// Resolve a handler.
    pub fn get(&self, kind: &str) -> Result<Arc<dyn JobHandler>> {
        self.handlers
            .read()
            .get(kind)
            .cloned()
            .ok_or_else(|| Error::scheduler(format!("no job handler for kind '{kind}'")))
    }

    /// List registered kinds.
    pub fn kinds(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }
}

/// No-op handler useful for smoke tests.
pub struct NoopHandler;

#[async_trait]
impl JobHandler for NoopHandler {
    fn kind(&self) -> &str {
        "noop"
    }

    async fn execute(&self, job: &JobDefinition) -> Result<Option<Value>> {
        Ok(Some(serde_json::json!({
            "job_id": job.id.to_string(),
            "status": "ok"
        })))
    }
}
