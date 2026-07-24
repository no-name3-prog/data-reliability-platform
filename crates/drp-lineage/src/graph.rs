//! In-memory directed lineage graph.

use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};

use drp_common::AssetId;
use drp_core::{LineageEdge, LineageNode};

/// Serializable subgraph snapshot for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSnapshot {
    /// Nodes.
    pub nodes: Vec<LineageNode>,
    /// Edges.
    pub edges: Vec<LineageEdge>,
}

/// Directed lineage graph keyed by [`AssetId`].
#[derive(Debug, Default)]
pub struct LineageGraph {
    graph: DiGraph<LineageNode, String>,
    index: HashMap<AssetId, NodeIndex>,
    edges: Vec<LineageEdge>,
}

impl LineageGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure a node exists for the asset.
    pub fn upsert_node(&mut self, node: LineageNode) {
        if let Some(&idx) = self.index.get(&node.asset_id) {
            self.graph[idx] = node;
        } else {
            let id = node.asset_id;
            let idx = self.graph.add_node(node);
            self.index.insert(id, idx);
        }
    }

    /// Add a directed edge (idempotent on from/to/kind).
    pub fn add_edge(&mut self, edge: LineageEdge) {
        let from_idx = match self.index.get(&edge.from) {
            Some(i) => *i,
            None => {
                let n = LineageNode {
                    asset_id: edge.from,
                    label: edge.from.to_string(),
                    node_type: None,
                };
                let idx = self.graph.add_node(n);
                self.index.insert(edge.from, idx);
                idx
            }
        };
        let to_idx = match self.index.get(&edge.to) {
            Some(i) => *i,
            None => {
                let n = LineageNode {
                    asset_id: edge.to,
                    label: edge.to.to_string(),
                    node_type: None,
                };
                let idx = self.graph.add_node(n);
                self.index.insert(edge.to, idx);
                idx
            }
        };

        let exists = self
            .edges
            .iter()
            .any(|e| e.from == edge.from && e.to == edge.to && e.kind == edge.kind);
        if !exists {
            self.graph.add_edge(from_idx, to_idx, edge.kind.clone());
            self.edges.push(edge);
        }
    }

    /// Full snapshot.
    pub fn snapshot(&self) -> LineageSnapshot {
        LineageSnapshot {
            nodes: self.graph.node_weights().cloned().collect(),
            edges: self.edges.clone(),
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

    fn walk(&self, asset_id: &AssetId, max_depth: u32, dir: Direction) -> LineageSnapshot {
        let Some(&start) = self.index.get(asset_id) else {
            return LineageSnapshot {
                nodes: vec![],
                edges: vec![],
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

        let edges = self
            .edges
            .iter()
            .filter(|e| edge_set.contains(&(e.from, e.to, e.kind.clone())))
            .cloned()
            .collect();

        LineageSnapshot { nodes, edges }
    }

    /// Node count.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
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
            g.upsert_node(LineageNode {
                asset_id: id,
                label: label.into(),
                node_type: None,
            });
        }
        g.add_edge(LineageEdge::transforms(a, b));
        g.add_edge(LineageEdge::transforms(b, c));
        let snap = g.downstream(&a, 10);
        assert_eq!(snap.nodes.len(), 3);
        assert_eq!(snap.edges.len(), 2);
    }
}
