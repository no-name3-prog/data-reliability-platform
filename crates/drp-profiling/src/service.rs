//! Profiling service.

use std::sync::Arc;

use tracing::instrument;

use drp_common::{AssetId, Error, Result};
use drp_core::{DatasetProfile, EventBus, PlatformEvent, PluginContext, PluginRegistry};
use drp_storage::Store;

/// Orchestrates sampling + profiling + persistence.
#[derive(Clone)]
pub struct ProfilingService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    events: EventBus,
    default_profiler: String,
    sample_size: usize,
}

impl ProfilingService {
    /// Create a profiling service.
    pub fn new(
        store: Arc<dyn Store>,
        plugins: PluginRegistry,
        events: EventBus,
        sample_size: usize,
    ) -> Self {
        Self {
            store,
            plugins,
            events,
            default_profiler: "basic".into(),
            sample_size,
        }
    }

    /// Profile an asset using the given connector and profiler plugins.
    #[instrument(skip(self), fields(asset_id = %asset_id))]
    pub async fn profile_asset(
        &self,
        asset_id: &AssetId,
        connector_id: &str,
        profiler_id: Option<&str>,
    ) -> Result<DatasetProfile> {
        let asset = self
            .store
            .get_asset(asset_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("asset {asset_id}")))?;

        let connector = self.plugins.connector(connector_id)?;
        let profiler_id = profiler_id.unwrap_or(&self.default_profiler);
        let profiler = self.plugins.profiler(profiler_id)?;
        let ctx = PluginContext::new();

        let rows = connector
            .sample_rows(&asset, self.sample_size, &ctx)
            .await?;
        let profile = profiler.profile(&asset, &rows, &ctx).await?;
        let saved = self.store.save_profile(profile).await?;

        self.events
            .publish(PlatformEvent::ProfileCompleted {
                asset_id: *asset_id,
                run_id: saved.run_id,
            })
            .await;

        Ok(saved)
    }

    /// Fetch the latest stored profile for an asset.
    pub async fn latest_profile(&self, asset_id: &AssetId) -> Result<Option<DatasetProfile>> {
        self.store.latest_profile(asset_id).await
    }
}
