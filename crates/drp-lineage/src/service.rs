//! Lineage service: SQL registration, graph mutations, impact analysis.

use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;

use crate::graph::{LineageGraph, LineageSnapshot};
use crate::sql_parse::{extract_lineage_from_sql, SqlLineageExtract};
use drp_common::{AssetId, Error, Result};
use drp_core::{
    ColumnLineageEdge, ColumnRef, ImpactReport, ImpactTrigger, LineageEdge, LineageNode,
    LineageNodeKind,
};

/// Result of parsing SQL and applying it to the lineage graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SqlIngestResult {
    /// Target asset if resolved/created.
    pub target_asset_id: Option<AssetId>,
    /// Target FQN.
    pub target_fqn: Option<String>,
    /// Source asset ids linked.
    pub source_asset_ids: Vec<AssetId>,
    /// Table-level edges added.
    pub table_edges_added: usize,
    /// Column-level edges added.
    pub column_edges_added: usize,
    /// Per-statement extracts (debug).
    pub statements: usize,
}

/// Thread-safe lineage service.
#[derive(Clone)]
pub struct LineageService {
    graph: Arc<RwLock<LineageGraph>>,
    max_depth: u32,
}

impl Default for LineageService {
    fn default() -> Self {
        Self::new(20)
    }
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

    /// Add a table-level edge.
    pub fn add_edge(&self, edge: LineageEdge) {
        self.graph.write().add_edge(edge);
    }

    /// Add a column-level edge.
    pub fn add_column_edge(&self, edge: ColumnLineageEdge) {
        self.graph.write().add_column_edge(edge);
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

    /// Column-level upstream.
    pub fn column_upstream(
        &self,
        asset_id: &AssetId,
        column: &str,
        depth: Option<u32>,
    ) -> Vec<ColumnLineageEdge> {
        self.graph
            .read()
            .column_upstream(asset_id, column, depth.unwrap_or(self.max_depth))
    }

    /// Column-level downstream.
    pub fn column_downstream(
        &self,
        asset_id: &AssetId,
        column: &str,
        depth: Option<u32>,
    ) -> Vec<ColumnLineageEdge> {
        self.graph
            .read()
            .column_downstream(asset_id, column, depth.unwrap_or(self.max_depth))
    }

    /// Register an asset node from catalog metadata.
    pub fn register_asset(&self, asset_id: AssetId, label: impl Into<String>) {
        let label = label.into();
        self.upsert_node(LineageNode::table(asset_id, label).with_kind(LineageNodeKind::Table));
    }

    /// Register a node with explicit kind (dataset / dashboard / pipeline / …).
    pub fn register_node(
        &self,
        asset_id: AssetId,
        label: impl Into<String>,
        kind: LineageNodeKind,
        fqn: Option<String>,
    ) {
        let mut node = LineageNode::table(asset_id, label).with_kind(kind);
        if let Some(f) = fqn {
            node = node.with_fqn(f);
        }
        self.upsert_node(node);
    }

    /// Resolve FQN to asset id if known.
    pub fn resolve_fqn(&self, fqn: &str) -> Option<AssetId> {
        self.graph.read().resolve_fqn(fqn)
    }

    /// Parse SQL and register table- + column-level lineage.
    ///
    /// * `default_target` — optional target asset when SQL has no CREATE/INSERT target.
    /// * Unknown tables are auto-registered as provisional table nodes.
    pub fn ingest_sql(
        &self,
        sql: &str,
        default_target: Option<AssetId>,
    ) -> Result<SqlIngestResult> {
        let extracts = extract_lineage_from_sql(sql)?;
        let mut total_table = 0usize;
        let mut total_col = 0usize;
        let mut last_target = None;
        let mut last_target_fqn = None;
        let mut all_sources = Vec::new();

        for extract in &extracts {
            let (t_edges, c_edges, target_id, sources) =
                self.apply_extract(extract, default_target)?;
            total_table += t_edges;
            total_col += c_edges;
            last_target = target_id.or(last_target);
            last_target_fqn = extract.target.clone().or(last_target_fqn);
            all_sources.extend(sources);
        }

        all_sources.sort_by_key(|id| id.to_string());
        all_sources.dedup();

        info!(
            statements = extracts.len(),
            table_edges = total_table,
            column_edges = total_col,
            "SQL lineage ingested"
        );

        Ok(SqlIngestResult {
            target_asset_id: last_target,
            target_fqn: last_target_fqn,
            source_asset_ids: all_sources,
            table_edges_added: total_table,
            column_edges_added: total_col,
            statements: extracts.len(),
        })
    }

    fn apply_extract(
        &self,
        extract: &SqlLineageExtract,
        default_target: Option<AssetId>,
    ) -> Result<(usize, usize, Option<AssetId>, Vec<AssetId>)> {
        let mut g = self.graph.write();
        let mut table_edges = 0usize;
        let mut col_edges = 0usize;

        let target_id = if let Some(ref t) = extract.target {
            Some(Self::ensure_named(&mut g, t, LineageNodeKind::View))
        } else {
            default_target
        };

        let mut source_ids = Vec::new();
        for src in &extract.sources {
            let id = Self::ensure_named(&mut g, src, LineageNodeKind::Table);
            source_ids.push(id);
            if let Some(tid) = target_id {
                if id != tid {
                    let before = g.edge_count();
                    g.add_edge(LineageEdge::sql_derived(id, tid, extract.sql.clone()));
                    if g.edge_count() > before {
                        table_edges += 1;
                    }
                }
            }
        }

        if let Some(tid) = target_id {
            for m in &extract.column_mappings {
                for (src_table, src_col) in &m.sources {
                    let from_asset = if let Some(tname) = src_table {
                        Self::ensure_named(&mut g, tname, LineageNodeKind::Table)
                    } else if source_ids.len() == 1 {
                        source_ids[0]
                    } else {
                        // ambiguous unqualified column — skip column edge
                        continue;
                    };
                    let before = g.column_edge_count();
                    let mut edge = ColumnLineageEdge::maps_to(
                        ColumnRef::new(from_asset, src_col.clone()),
                        ColumnRef::new(tid, m.target_column.clone()),
                    );
                    if let Some(ref expr) = m.expression {
                        edge = edge.with_expression(expr.clone());
                    }
                    g.add_column_edge(edge);
                    if g.column_edge_count() > before {
                        col_edges += 1;
                    }
                }
            }
        }

        Ok((table_edges, col_edges, target_id, source_ids))
    }

    fn ensure_named(g: &mut LineageGraph, name: &str, kind: LineageNodeKind) -> AssetId {
        if let Some(id) = g.resolve_fqn(name) {
            // Upgrade kind if currently table and we want view for targets
            if let Some(mut node) = g.get_node(&id) {
                if matches!(kind, LineageNodeKind::View)
                    && matches!(node.kind, LineageNodeKind::Table)
                {
                    node.kind = LineageNodeKind::View;
                    node.node_type = Some("view".into());
                    g.upsert_node(node);
                }
            }
            return id;
        }
        let id = AssetId::new();
        g.upsert_node(
            LineageNode::table(id, name.to_string())
                .with_fqn(name.to_string())
                .with_kind(kind),
        );
        id
    }

    /// Link a dashboard node to tables it reads.
    pub fn register_dashboard(
        &self,
        dashboard_id: AssetId,
        name: impl Into<String>,
        reads: &[AssetId],
    ) {
        self.register_node(dashboard_id, name, LineageNodeKind::Dashboard, None);
        for src in reads {
            self.add_edge(LineageEdge::transforms(*src, dashboard_id).with_kind("dashboard_reads"));
        }
    }

    /// Link a pipeline that produces targets from sources.
    pub fn register_pipeline(
        &self,
        pipeline_id: AssetId,
        name: impl Into<String>,
        sources: &[AssetId],
        targets: &[AssetId],
    ) {
        self.register_node(pipeline_id, name, LineageNodeKind::Pipeline, None);
        for s in sources {
            self.add_edge(LineageEdge::transforms(*s, pipeline_id).with_kind("pipeline_reads"));
        }
        for t in targets {
            self.add_edge(LineageEdge::transforms(pipeline_id, *t).with_kind("pipeline_produces"));
        }
    }

    /// Impact when a table changes: downstream datasets, dashboards, pipelines.
    pub fn impact_table_change(&self, asset_id: &AssetId, depth: Option<u32>) -> ImpactReport {
        self.graph.read().impact(
            asset_id,
            ImpactTrigger::TableChange {
                asset_id: *asset_id,
            },
            depth.unwrap_or(self.max_depth),
        )
    }

    /// Impact when validation fails on an asset.
    pub fn impact_validation_failed(
        &self,
        asset_id: &AssetId,
        check_id: Option<String>,
        message: Option<String>,
        depth: Option<u32>,
    ) -> ImpactReport {
        self.graph.read().impact(
            asset_id,
            ImpactTrigger::ValidationFailed {
                asset_id: *asset_id,
                check_id,
                message,
            },
            depth.unwrap_or(self.max_depth),
        )
    }

    /// Stats helper for health endpoints.
    pub fn stats(&self) -> Result<(usize, usize)> {
        let g = self.graph.read();
        Ok((g.node_count(), g.edge_count()))
    }

    /// Extended stats including column edges.
    pub fn stats_detailed(&self) -> Result<(usize, usize, usize)> {
        let g = self.graph.read();
        Ok((g.node_count(), g.edge_count(), g.column_edge_count()))
    }

    /// Ensure asset exists or error.
    pub fn require_node(&self, asset_id: &AssetId) -> Result<LineageNode> {
        self.graph
            .read()
            .get_node(asset_id)
            .ok_or_else(|| Error::not_found(format!("lineage node {asset_id}")))
    }
}
