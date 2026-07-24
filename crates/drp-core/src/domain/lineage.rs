//! Lineage graph primitives.

use serde::{Deserialize, Serialize};

use drp_common::{AssetId, UtcTimestamp};

/// A node in the lineage graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Asset this node represents.
    pub asset_id: AssetId,
    /// Display label.
    pub label: String,
    /// Optional node type override.
    #[serde(default)]
    pub node_type: Option<String>,
}

/// Directed edge: `from` produces / feeds `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Upstream asset.
    pub from: AssetId,
    /// Downstream asset.
    pub to: AssetId,
    /// Edge kind.
    pub kind: String,
    /// When the edge was observed or declared.
    pub observed_at: UtcTimestamp,
}

impl LineageEdge {
    /// Create a transforms edge.
    pub fn transforms(from: AssetId, to: AssetId) -> Self {
        Self {
            from,
            to,
            kind: "transforms".into(),
            observed_at: UtcTimestamp::now(),
        }
    }
}
