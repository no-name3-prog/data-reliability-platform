//! Profiling service — auto-profile after discovery, history, and comparison.

use std::sync::Arc;

use tracing::{info, instrument, warn};

use drp_common::{AssetId, Error, Result, RunId};
use drp_core::{
    DatasetProfile, EventBus, PlatformEvent, PluginContext, PluginRegistry, ProfileDiff,
};
use drp_storage::Store;

/// Orchestrates sampling + profiling + history persistence.
#[derive(Clone)]
pub struct ProfilingService {
    store: Arc<dyn Store>,
    plugins: PluginRegistry,
    events: EventBus,
    default_profiler: String,
    sample_size: usize,
    /// When true, callers may use [`Self::profile_assets_batch`] after discovery.
    auto_profile: bool,
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
            auto_profile: true,
        }
    }

    /// Enable or disable automatic post-discovery profiling.
    pub fn with_auto_profile(mut self, enabled: bool) -> Self {
        self.auto_profile = enabled;
        self
    }

    /// Whether auto-profile after discovery is enabled.
    pub fn auto_profile_enabled(&self) -> bool {
        self.auto_profile
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
        let mut profile = profiler.profile(&asset, &rows, &ctx).await?;
        profile.connector = Some(connector_id.to_string());
        profile.profiler = Some(profiler_id.to_string());
        profile.sample_size = Some(self.sample_size as u64);
        profile.asset_fqn = Some(asset.fqn.clone());

        let saved = self.store.save_profile(profile).await?;

        self.events
            .publish(PlatformEvent::ProfileCompleted {
                asset_id: *asset_id,
                run_id: saved.run_id,
            })
            .await;

        info!(
            asset_id = %asset_id,
            run_id = %saved.run_id,
            rows = saved.row_count,
            columns = saved.columns.len(),
            "profile saved to history"
        );

        Ok(saved)
    }

    /// Profile many assets after discovery (best-effort; continues on individual failures).
    #[instrument(skip(self, asset_ids))]
    pub async fn profile_assets_batch(
        &self,
        asset_ids: &[AssetId],
        connector_id: &str,
    ) -> Vec<Result<DatasetProfile>> {
        if !self.auto_profile {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(asset_ids.len());
        for id in asset_ids {
            match self.profile_asset(id, connector_id, None).await {
                Ok(p) => out.push(Ok(p)),
                Err(e) => {
                    warn!(asset_id = %id, error = %e, "auto-profile failed");
                    out.push(Err(e));
                }
            }
        }
        out
    }

    /// Fetch the latest stored profile for an asset.
    pub async fn latest_profile(&self, asset_id: &AssetId) -> Result<Option<DatasetProfile>> {
        self.store.latest_profile(asset_id).await
    }

    /// Profile history newest-first.
    pub async fn profile_history(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<DatasetProfile>> {
        self.store.list_profile_history(asset_id, limit).await
    }

    /// Get a specific historical run.
    pub async fn get_profile_run(
        &self,
        asset_id: &AssetId,
        run_id: &RunId,
    ) -> Result<Option<DatasetProfile>> {
        self.store.get_profile_by_run(asset_id, run_id).await
    }

    /// Compare two historical runs (or latest vs previous when ids are omitted).
    pub async fn compare_profiles(
        &self,
        asset_id: &AssetId,
        baseline_run: Option<&RunId>,
        current_run: Option<&RunId>,
    ) -> Result<ProfileDiff> {
        let history = self.store.list_profile_history(asset_id, Some(50)).await?;
        if history.len() < 2 && baseline_run.is_none() {
            return Err(Error::validation(
                "need at least two profile runs to compare",
            ));
        }

        let current = if let Some(id) = current_run {
            history
                .iter()
                .find(|p| p.run_id == *id)
                .cloned()
                .ok_or_else(|| Error::not_found(format!("profile run {id}")))?
        } else {
            history
                .first()
                .cloned()
                .ok_or_else(|| Error::not_found("no profiles for asset"))?
        };

        let baseline = if let Some(id) = baseline_run {
            history
                .iter()
                .find(|p| p.run_id == *id)
                .cloned()
                .ok_or_else(|| Error::not_found(format!("baseline profile run {id}")))?
        } else {
            history
                .get(1)
                .cloned()
                .ok_or_else(|| Error::not_found("no baseline profile"))?
        };

        Ok(current.diff_from(&baseline))
    }
}
