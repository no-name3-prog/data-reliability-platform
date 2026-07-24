//! Plugin system — the primary extension mechanism.

mod registry;
mod traits;

pub use registry::PluginRegistry;
pub use traits::{
    ConnectorPlugin, NotificationPlugin, Plugin, PluginCapability, PluginContext, PluginInfo,
    ProfilerPlugin, ValidatorPlugin,
};
