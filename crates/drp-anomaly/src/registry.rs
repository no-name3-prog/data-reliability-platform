//! Built-in detector registration.

use std::sync::Arc;

use drp_core::PluginRegistry;

use crate::{NullSpikeDetector, ZScoreDetector};

/// Register built-in anomaly detectors.
pub fn register_builtin_detectors(registry: &PluginRegistry) {
    registry.register_anomaly_detector(Arc::new(NullSpikeDetector::new()));
    registry.register_anomaly_detector(Arc::new(ZScoreDetector::new()));
}
