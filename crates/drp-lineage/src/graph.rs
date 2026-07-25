//! In-memory directed lineage graph (table + column level).

use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};

use drp_common::AssetId;
use drp_core::{
    ColumnLineageEdge, ImpactReport, ImpactTrigger, ImpactedAsset, LineageEdge, LineageNode,
    LineageNodeKind, LineageSubgraphMeta,
};

/// Serializable subgraph snapshot for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSnapshot {
    /// Nodes.
    pub nodes: Vec<LineageNode>,
    /// Table-level edges.
    pub edges: Vec<LineageEdge>,
    /// Column-level edges (optional; may be filtered to subgraph).
    #[serde(default)]
    pub column_edges: Vec<ColumnLineageEdge>,
}

/// Directed lineage graph keyed by [`AssetId`].
#[derive(Debug, Default)]
pub struct LineageGraph {
    graph: DiGraph<LineageNode, String>,
    index: HashMap<AssetId, NodeIndex>,
    /// FQN (lowercase) → asset id for SQL resolution.
    fqn_index: HashMap<String, AssetId>,
    edges: Vec<LineageEdge>,
    column_edges: Vec<ColumnLineageEdge>,
}

impl LineageGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure a node exists for the asset.
    pub fn upsert_node(&mut self, node: LineageNode) {
        if let Some(fqn) = node.fqn.as_ref().or(Some(&node.label)) {
            self.fqn_index.insert(normalize_fqn(fqn), node.asset_id);
        }
        if let Some(&idx) = self.index.get(&node.asset_id) {
            self.graph[idx] = node;
        } else {
            let id = node.asset_id;
            let idx = self.graph.add_node(node);
            self.index.insert(id, idx);
        }
    }

    /// Resolve FQN (case-insensitive) to asset id.
    pub fn resolve_fqn(&self, fqn: &str) -> Option<AssetId> {
        let key = normalize_fqn(fqn);
        if let Some(id) = self.fqn_index.get(&key) {
            return Some(*id);
        }
        // Try bare name match (suffix).
        let bare = key.rsplit('.').next().unwrap_or(&key);
        self.fqn_index
            .iter()
            .find(|(k, _)| k.rsplit('.').next() == Some(bare) || *k == bare)
            .map(|(_, id)| *id)
    }

    /// Add a directed edge (idempotent on from/to/kind).
    pub fn add_edge(&mut self, edge: LineageEdge) {
        self.ensure_placeholder(edge.from);
        self.ensure_placeholder(edge.to);
        let from_idx = self.index[&edge.from];
        let to_idx = self.index[&edge.to];

        let exists = self
            .edges
            .iter()
            .any(|e| e.from == edge.from && e.to == edge.to && e.kind == edge.kind);
        if !exists {
            self.graph.add_edge(from_idx, to_idx, edge.kind.clone());
            self.edges.push(edge);
        }
    }

    /// Add a column-level edge (idempotent on from/to columns).
    pub fn add_column_edge(&mut self, edge: ColumnLineageEdge) {
        self.ensure_placeholder(edge.from.asset_id);
        self.ensure_placeholder(edge.to.asset_id);
        let exists = self.column_edges.iter().any(|e| {
            e.from.asset_id == edge.from.asset_id
                && e.from.column == edge.from.column
                && e.to.asset_id == edge.to.asset_id
                && e.to.column == edge.to.column
        });
        if !exists {
            self.column_edges.push(edge);
        }
    }

    fn ensure_placeholder(&mut self, asset_id: AssetId) {
        if self.index.contains_key(&asset_id) {
            return;
        }
        let n = LineageNode::table(asset_id, asset_id.to_string());
        let idx = self.graph.add_node(n);
        self.index.insert(asset_id, idx);
    }

    /// Full snapshot.
    pub fn snapshot(&self) -> LineageSnapshot {
        LineageSnapshot {
            nodes: self.graph.node_weights().cloned().collect(),
            edges: self.edges.clone(),
            column_edges: self.column_edges.clone(),
        }
    }

    /// BFS upstream (producers) up to `max_depth`.
    pub fn upstream(&self, asset_id: &AssetId, max_depth: u32) -> LineageSnapshot {
        self.walk(asset_id, max_depth, Direction::Incoming)
    }

    /// BFS downstream (consumers) up to `max_depth`.
    pub fn downstream(&self, asset_id: &AssetId, max_depth: u32) -> LineageSnapshot {
        self.walk(asset_id, max_depth, Direction::Outgoing)
    }

    /// Column-level upstream: columns that feed `asset_id.column`.
    pub fn column_upstream(
        &self,
        asset_id: &AssetId,
        column: &str,
        max_depth: u32,
    ) -> Vec<ColumnLineageEdge> {
        self.walk_columns(asset_id, column, max_depth, true)
    }

    /// Column-level downstream.
    pub fn column_downstream(
        &self,
        asset_id: &AssetId,
        column: &str,
        max_depth: u32,
    ) -> Vec<ColumnLineageEdge> {
        self.walk_columns(asset_id, column, max_depth, false)
    }

    fn walk_columns(
        &self,
        asset_id: &AssetId,
        column: &str,
        max_depth: u32,
        upstream: bool,
    ) -> Vec<ColumnLineageEdge> {
        let col_key = |a: &AssetId, c: &str| format!("{}::{}", a, c.to_ascii_lowercase());
        let start = col_key(asset_id, column);
        let mut visited = HashSet::new();
        let mut out = Vec::new();
        let mut q = VecDeque::new();
        q.push_back((start.clone(), 0u32));
        visited.insert(start);

        while let Some((key, depth)) = q.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for e in &self.column_edges {
                let (src, dst) = (
                    col_key(&e.from.asset_id, &e.from.column),
                    col_key(&e.to.asset_id, &e.to.column),
                );
                let (from_key, to_key, edge) = if upstream {
                    // walk against edge direction: to → from
                    if dst == key {
                        (dst, src, e)
                    } else {
                        continue;
                    }
                } else if src == key {
                    (src, dst, e)
                } else {
                    continue;
                };
                let _ = from_key;
                out.push(edge.clone());
                if visited.insert(to_key.clone()) {
                    q.push_back((to_key, depth + 1));
                }
            }
        }
        out
    }

    fn walk(&self, asset_id: &AssetId, max_depth: u32, dir: Direction) -> LineageSnapshot {
        let Some(&start) = self.index.get(asset_id) else {
            return LineageSnapshot {
                nodes: vec![],
                edges: vec![],
                column_edges: vec![],
            };
        };

        let mut visited = HashSet::new();
        let mut nodes = Vec::new();
        let mut edge_set = HashSet::new();
        let mut q = VecDeque::new();
        q.push_back((start, 0u32));
        visited.insert(start);

        while let Some((idx, depth)) = q.pop_front() {
            nodes.push(self.graph[idx].clone());
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges_directed(idx, dir) {
                let neighbor = match dir {
                    Direction::Outgoing => edge.target(),
                    Direction::Incoming => edge.source(),
                };
                let from_id = self.graph[edge.source()].asset_id;
                let to_id = self.graph[edge.target()].asset_id;
                edge_set.insert((from_id, to_id, edge.weight().clone()));
                if visited.insert(neighbor) {
                    q.push_back((neighbor, depth + 1));
                }
            }
        }

        let asset_ids: HashSet<AssetId> = nodes.iter().map(|n| n.asset_id).collect();
        let edges = self
            .edges
            .iter()
            .filter(|e| edge_set.contains(&(e.from, e.to, e.kind.clone())))
            .cloned()
            .collect();
        let column_edges = self
            .column_edges
            .iter()
            .filter(|e| asset_ids.contains(&e.from.asset_id) && asset_ids.contains(&e.to.asset_id))
            .cloned()
            .collect();

        LineageSnapshot {
            nodes,
            edges,
            column_edges,
        }
    }

    /// Downstream impact analysis grouped by node kind.
    pub fn impact(&self, root: &AssetId, trigger: ImpactTrigger, max_depth: u32) -> ImpactReport {
        let snap = self.downstream(root, max_depth);
        // depth map via BFS
        let depths = self.downstream_depths(root, max_depth);
        let mut report = ImpactReport::empty(trigger, *root);

        for node in &snap.nodes {
            if node.asset_id == *root {
                continue;
            }
            let depth = depths.get(&node.asset_id).copied().unwrap_or(1);
            let item = ImpactedAsset {
                asset_id: node.asset_id,
                label: node.label.clone(),
                kind: node.kind,
                depth,
                via: vec![],
            };
            match node.kind {
                LineageNodeKind::Table | LineageNodeKind::View | LineageNodeKind::File => {
                    report.tables.push(item)
                }
                LineageNodeKind::Dataset => report.datasets.push(item),
                LineageNodeKind::Dashboard => report.dashboards.push(item),
                LineageNodeKind::Pipeline => report.pipelines.push(item),
                LineageNodeKind::Other => report.other.push(item),
            }
        }
        report.subgraph = Some(LineageSubgraphMeta {
            nodes: snap.nodes,
            edges: snap.edges,
            column_edges: snap.column_edges,
        });
        report
    }

    fn downstream_depths(&self, root: &AssetId, max_depth: u32) -> HashMap<AssetId, u32> {
        let mut depths = HashMap::new();
        let Some(&start) = self.index.get(root) else {
            return depths;
        };
        let mut q = VecDeque::new();
        q.push_back((start, 0u32));
        depths.insert(*root, 0);
        while let Some((idx, depth)) = q.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                let neighbor = edge.target();
                let id = self.graph[neighbor].asset_id;
                if let std::collections::hash_map::Entry::Vacant(e) = depths.entry(id) {
                    e.insert(depth + 1);
                    q.push_back((neighbor, depth + 1));
                }
            }
        }
        depths
    }

    /// Node count.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Column edge count.
    pub fn column_edge_count(&self) -> usize {
        self.column_edges.len()
    }

    /// Get node by asset id.
    pub fn get_node(&self, asset_id: &AssetId) -> Option<LineageNode> {
        self.index.get(asset_id).map(|idx| self.graph[*idx].clone())
    }
}

fn normalize_fqn(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use drp_core::LineageEdge;

    #[test]
    fn downstream_traversal() {
        let mut g = LineageGraph::new();
        let a = AssetId::new();
        let b = AssetId::new();
        let c = AssetId::new();
        for (id, label) in [(a, "a"), (b, "b"), (c, "c")] {
            g.upsert_node(LineageNode::table(id, label));
        }
        g.add_edge(LineageEdge::transforms(a, b));
        g.add_edge(LineageEdge::transforms(b, c));
        let snap = g.downstream(&a, 10);
        assert_eq!(snap.nodes.len(), 3);
        assert_eq!(snap.edges.len(), 2);
    }

    #[test]
    fn impact_groups_dashboards_and_pipelines() {
        let mut g = LineageGraph::new();
        let table = AssetId::new();
        let dash = AssetId::new();
        let pipe = AssetId::new();
        g.upsert_node(LineageNode::table(table, "raw.orders"));
        g.upsert_node(
            LineageNode::table(dash, "Sales Dashboard").with_kind(LineageNodeKind::Dashboard),
        );
        g.upsert_node(LineageNode::table(pipe, "etl_orders").with_kind(LineageNodeKind::Pipeline));
        g.add_edge(LineageEdge::transforms(table, dash).with_kind("dashboard_reads"));
        g.add_edge(LineageEdge::transforms(table, pipe).with_kind("pipeline_produces"));
        let report = g.impact(&table, ImpactTrigger::TableChange { asset_id: table }, 5);
        assert_eq!(report.dashboards.len(), 1);
        assert_eq!(report.pipelines.len(), 1);
    }
}
