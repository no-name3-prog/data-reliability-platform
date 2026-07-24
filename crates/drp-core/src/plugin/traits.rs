//! Extension-point traits for the Data Reliability Platform.
//!
//! # Design principles
//!
//! 1. **Traits live in `drp-core`** — the stable ABI for plugins.
//! 2. **Implementations live in other crates** — never force core edits to add a plugin.
//! 3. **Registration is explicit** at the composition root (`drp-api`) or via a
//!    crate-local `register_*` helper that takes [`PluginRegistry`].
//! 4. **Object-safe + async** via [`async_trait`] so plugins can be stored as
//!    `Arc<dyn Trait>`.
//!
//! See `docs/plugin-architecture.md` for the full guide.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    AiRequest, AiResponse, AnomalyReport, Asset, CatalogTree, CheckDefinition, CheckResult,
    DatasetProfile,
};
use drp_common::{Result, SourceLocation};

/// Declared capabilities a plugin may advertise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Reads/writes external data systems.
    Connector,
    /// Computes statistical profiles.
    Profiler,
    /// Executes data-quality checks / validation rules.
    Validator,
    /// Detects anomalies / drift / outliers.
    AnomalyDetector,
    /// Delivers alerts / messages.
    Notification,
    /// Large language model / AI completions.
    AiProvider,
    /// Custom / multi-purpose plugin.
    Extension,
}

/// Static metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Stable machine id (e.g. `"postgres"`, `"not_null"`, `"zscore"`).
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
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
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

    /// Read an optional string config key.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(|v| v.as_str())
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Base plugin identity — every plugin implements this.
pub trait Plugin: Send + Sync {
    /// Plugin metadata.
    fn info(&self) -> &PluginInfo;
}

// ---------------------------------------------------------------------------
// Extension points
// ---------------------------------------------------------------------------

/// Data-source connector plugin.
///
/// Implement in a separate crate (e.g. `drp-connector-postgres`) and register
/// with [`crate::PluginRegistry::register_connector`].
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

    /// Hierarchical discovery (databases → schemas → tables → columns).
    ///
    /// Default implementation wraps [`Self::discover`] into a single synthetic
    /// `default.public` namespace so simple connectors stay easy to implement.
    async fn discover_catalog(
        &self,
        location: &SourceLocation,
        ctx: &PluginContext,
    ) -> Result<CatalogTree> {
        use crate::domain::{CatalogDatabase, CatalogSchema, CatalogTable};

        let assets = self.discover(location, ctx).await?;
        let mut tree = CatalogTree::new(self.info().id.clone(), location.clone());
        let mut db = CatalogDatabase::new("default");
        let mut schema = CatalogSchema::new("public");
        for a in assets {
            let mut t = CatalogTable {
                name: a.name.clone(),
                kind: a.kind,
                fqn: a.fqn.clone(),
                columns: a.columns.clone(),
                row_count_estimate: None,
                properties: a.tags.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            };
            t.properties
                .insert("source_uri".into(), a.location.uri.clone());
            schema.tables.push(t);
        }
        db.schemas.push(schema);
        tree.databases.push(db);
        Ok(tree)
    }
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

/// Validation / data-quality rule plugin.
///
/// One plugin id maps to one rule family (e.g. `not_null`, `unique`). Parameters
/// for a concrete check are carried on [`CheckDefinition::params`].
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

/// Anomaly / drift / outlier detector plugin.
///
/// Detectors consume sampled rows (and optionally prior profiles) and emit an
/// [`AnomalyReport`]. Core never hard-codes detector algorithms.
#[async_trait]
pub trait AnomalyDetectorPlugin: Plugin {
    /// Run detection for an asset sample.
    async fn detect(
        &self,
        asset: &Asset,
        rows: &[IndexMap<String, Value>],
        ctx: &PluginContext,
    ) -> Result<AnomalyReport>;
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

/// AI / LLM provider plugin.
///
/// Used for summarization, root-cause hints, contract suggestions, etc.
/// Implementations may call remote APIs; the core engine only sees this trait.
#[async_trait]
pub trait AiProviderPlugin: Plugin {
    /// Complete a chat / prompt request.
    async fn complete(&self, request: &AiRequest, ctx: &PluginContext) -> Result<AiResponse>;

    /// Optional health probe (default: always healthy).
    async fn health(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }
}

/// Optional convenience: a crate can implement this to register many plugins at once.
///
/// ```ignore
/// pub struct MyBundle;
/// impl PluginBundle for MyBundle {
///     fn register(&self, registry: &PluginRegistry) {
///         registry.register_connector(Arc::new(MyConnector::new()));
///     }
/// }
/// ```
pub trait PluginBundle: Send + Sync {
    /// Register all plugins provided by this bundle.
    fn register(&self, registry: &crate::PluginRegistry);
}
