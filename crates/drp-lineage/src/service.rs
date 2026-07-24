//! Lineage service with interior mutability.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::graph::{LineageGraph, LineageSnapshot};
use drp_common::{AssetId, Result};
use drp_core::{LineageEdge, LineageNode};

/// Thread-safe lineage service.
#[derive(Clone, Default)]
pub struct LineageService {
    graph: Arc<RwLock<LineageGraph>>,
    max_depth: u32,
}

impl LineageService {
    /// Create a lineage service with a default max traversal depth.
    pub fn new(max_depth: u32) -> Self {
        Self {
            graph: Arc::new(RwLock::new(LineageGraph::new())),
            max_depth,
        }
    }

    /// Upsert a node.
    pub fn upsert_node(&self, node: LineageNode) {
        self.graph.write().upsert_node(node);
    }

    /// Add an edge.
    pub fn add_edge(&self, edge: LineageEdge) {
        self.graph.write().add_edge(edge);
    }

    /// Full graph snapshot.
    pub fn snapshot(&self) -> LineageSnapshot {
        self.graph.read().snapshot()
    }

    /// Upstream impact analysis.
    pub fn upstream(&self, asset_id: &AssetId, depth: Option<u32>) -> LineageSnapshot {
        self.graph
            .read()
            .upstream(asset_id, depth.unwrap_or(self.max_depth))
    }

    /// Downstream impact analysis.
    pub fn downstream(&self, asset_id: &AssetId, depth: Option<u32>) -> LineageSnapshot {
        self.graph
            .read()
            .downstream(asset_id, depth.unwrap_or(self.max_depth))
    }

    /// Register an asset node from catalog metadata.
    pub fn register_asset(&self, asset_id: AssetId, label: impl Into<String>) {
        self.upsert_node(LineageNode {
            asset_id,
            label: label.into(),
            node_type: None,
        });
    }

    /// Stats helper for health endpoints.
    pub fn stats(&self) -> Result<(usize, usize)> {
        let g = self.graph.read();
        Ok((g.node_count(), g.edge_count()))
    }
}
