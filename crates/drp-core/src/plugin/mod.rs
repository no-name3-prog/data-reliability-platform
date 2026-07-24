//! Plugin system — the primary extension mechanism.
//!
//! # Adding a plugin without changing core
//!
//! 1. Create a new crate (workspace or external).
//! 2. Depend on `drp-core` (+ `drp-common` as needed).
//! 3. Implement the relevant trait (`ConnectorPlugin`, `ValidatorPlugin`, …).
//! 4. Export `pub fn register(registry: &PluginRegistry)`.
//! 5. Call that function once from the composition root (`drp-api`), **not** from
//!    feature services.
//!
//! Full guide: `docs/plugin-architecture.md`.

mod registry;
mod traits;

pub use registry::PluginRegistry;
pub use traits::{
    AiProviderPlugin, AnomalyDetectorPlugin, ConnectorPlugin, NotificationPlugin, Plugin,
    PluginBundle, PluginCapability, PluginContext, PluginInfo, ProfilerPlugin, ValidatorPlugin,
};
