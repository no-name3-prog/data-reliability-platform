//! Extension-point traits.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{Asset, CheckDefinition, CheckResult, DatasetProfile};
use drp_common::{Result, SourceLocation};

/// Declared capabilities a plugin may advertise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Reads/writes external data systems.
    Connector,
    /// Computes statistical profiles.
    Profiler,
    /// Executes data-quality checks.
    Validator,
    /// Delivers alerts / messages.
    Notification,
    /// Custom / multi-purpose plugin.
    Extension,
}

/// Static metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Stable machine id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version of the plugin implementation.
    pub version: String,
    /// Short description.
    pub description: String,
    /// Capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,
}

impl PluginInfo {
    /// Builder helper.
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: String::new(),
            capabilities: Vec::new(),
        }
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: PluginCapability) -> Self {
        self.capabilities.push(cap);
        self
    }
}

/// Runtime context passed to plugins.
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Free-form configuration for this invocation.
    pub config: IndexMap<String, Value>,
    /// Tenant / workspace boundary (optional).
    pub tenant_id: Option<String>,
}

impl PluginContext {
    /// Empty context.
    pub fn new() -> Self {
        Self {
            config: IndexMap::new(),
            tenant_id: None,
        }
    }

    /// With a config map.
    pub fn with_config(config: IndexMap<String, Value>) -> Self {
        Self {
            config,
            tenant_id: None,
        }
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Base plugin identity.
pub trait Plugin: Send + Sync {
    /// Plugin metadata.
    fn info(&self) -> &PluginInfo;
}

/// Data-source connector plugin.
#[async_trait]
pub trait ConnectorPlugin: Plugin {
    /// Probe connectivity / credentials.
    async fn test_connection(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<()>;

    /// Discover assets at a location.
    async fn discover(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<Vec<Asset>>;

    /// Fetch a sample of rows as JSON objects.
    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>>;
}

/// Profiling plugin.
#[async_trait]
pub trait ProfilerPlugin: Plugin {
    /// Profile an asset given sampled rows.
    async fn profile(
        &self,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<DatasetProfile>;
}

/// Validation / check plugin.
#[async_trait]
pub trait ValidatorPlugin: Plugin {
    /// Execute a check definition against data.
    async fn validate(
        &self,
        check: &CheckDefinition,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<CheckResult>;
}

/// Notification / alert channel plugin.
#[async_trait]
pub trait NotificationPlugin: Plugin {
    /// Deliver a notification payload.
    async fn send(
        &self,
        subject: &str,
        body: &str,
        metadata: &IndexMap<String, Value>,
        ctx: &PluginContext,
    ) -> Result<()>;
}
