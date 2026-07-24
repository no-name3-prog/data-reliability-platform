//! Built-in profiler registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::BasicProfiler;

/// Register built-in profilers.
pub fn register_builtin_profilers(registry: &PluginRegistry) {
    registry.register_profiler(Arc::new(BasicProfiler::new()));
}
