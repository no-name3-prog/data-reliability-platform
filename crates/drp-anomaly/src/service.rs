//! Anomaly service — resolves detector plugins by id.

use std::sync::Arc;

use tracing::instrument;

use drp_common::{AssetId, Error, Result};
use drp_core::{AnomalyReport, PluginContext, PluginRegistry};
use drp_storage::Store;

/// Orchestrates sampling (via connector) + anomaly detection.
#[derive(Clone)]
pub struct AnomalyService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    sample_size: usize,
    default_detector: String,
}

impl AnomalyService {
    /// Create an anomaly service.
    pub fn new(store: Arc<dyn Store>, plugins: PluginRegistry, sample_size: usize) -> Self {
        Self {
            store,
            plugins,
            sample_size,
            default_detector: "null_spike".into(),
        }
    }

    /// Run a detector against an asset using a connector for samples.
    #[instrument(skip(self), fields(asset_id = %asset_id))]
    pub async fn detect(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
        detector_id: Option<&str>,
    ) -> Result<AnomalyReport> {
        let asset = self
            .store
            .get_asset(asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {asset_id}")))?;
        let connector = self.plugins.connector(connector_id)?;
        let detector_id = detector_id.unwrap_or(&self.default_detector);
        let detector = self.plugins.anomaly_detector(detector_id)?;
        let ctx = PluginContext::new();
        let rows = connector
            .sample_rows(&asset, self.sample_size, &ctx)
            .await?;
        detector.detect(&asset, &rows, &ctx).await
    }
}
