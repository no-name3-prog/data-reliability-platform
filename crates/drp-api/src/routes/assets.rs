//! Asset catalog routes.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use drp_common::{AssetKind, SourceLocation};
use drp_core::Asset;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DiscoverRequest {
    connector: String,
    uri: String,
    #[serde(default)]
    properties: indexmap::IndexMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    fqn: String,
    name: String,
    kind: AssetKind,
    connector: String,
    uri: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssetListResponse {
    items: Vec<Asset>,
    count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/assets", get(list_assets).post(create_asset))
        .route("/v1/assets/discover", post(discover))
        .route("/v1/assets/{id}", get(get_asset))
}

async fn list_assets(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<AssetListResponse>> {
    let items = state.metadata.list_assets(q.limit).await?;
    let count = items.len();
    Ok(Json(AssetListResponse { items, count }))
}

async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Asset>> {
    let asset_id = id.parse().map_err(ApiError::from)?;
    Ok(Json(state.metadata.get_asset(&asset_id).await?))
}

async fn create_asset(
    State(state): State<AppState>,
    Json(body): Json<CreateAssetRequest>,
) -> ApiResult<Json<Asset>> {
    let mut asset = Asset::new(
        body.fqn,
        body.name,
        body.kind,
        SourceLocation::new(body.connector, body.uri),
    );
    asset.description = body.description;
    let saved = state.metadata.upsert_asset(asset).await?;
    state.lineage.register_asset(saved.id, saved.fqn.clone());
    Ok(Json(saved))
}

async fn discover(
    State(state): State<AppState>,
    Json(body): Json<DiscoverRequest>,
) -> ApiResult<Json<AssetListResponse>> {
    let mut location = SourceLocation::new(body.connector.clone(), body.uri);
    location.properties = body.properties;
    let items = state
        .metadata
        .discover_and_register(&body.connector, location)
        .await?;
    for a in &items {
        state.lineage.register_asset(a.id, a.fqn.clone());
    }
    // Auto-profile every discovered dataset
    let ids: Vec<_> = items.iter().map(|a| a.id).collect();
    let _profiles = state
        .profiling
        .profile_assets_batch(&ids, &body.connector)
        .await;
    let count = items.len();
    Ok(Json(AssetListResponse { items, count }))
}
