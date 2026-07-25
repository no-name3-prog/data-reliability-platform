//! Lineage graph primitives: table-level and column-level edges, impact analysis.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use drp_common::{AssetId, UtcTimestamp};

/// Logical role of a lineage node in the dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageNodeKind {
    /// Relational table / physical dataset.
    #[default]
    Table,
    /// SQL view / virtual table.
    View,
    /// Named dataset / semantic model.
    Dataset,
    /// BI dashboard / report.
    Dashboard,
    /// ETL / orchestration pipeline / job.
    Pipeline,
    /// File / object path.
    File,
    /// Other / unknown.
    Other,
}

impl LineageNodeKind {
    /// Stable machine name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::View => "view",
            Self::Dataset => "dataset",
            Self::Dashboard => "dashboard",
            Self::Pipeline => "pipeline",
            Self::File => "file",
            Self::Other => "other",
        }
    }
}

/// A node in the lineage graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Asset this node represents.
    pub asset_id: AssetId,
    /// Display label (often FQN).
    pub label: String,
    /// Fully-qualified name when known.
    #[serde(default)]
    pub fqn: Option<String>,
    /// Node kind (table, dashboard, pipeline, …).
    #[serde(default)]
    pub kind: LineageNodeKind,
    /// Optional free-form type string (legacy / display).
    #[serde(default)]
    pub node_type: Option<String>,
    /// Extra metadata.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

impl LineageNode {
    /// Create a table node.
    pub fn table(asset_id: AssetId, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            asset_id,
            fqn: Some(label.clone()),
            label,
            kind: LineageNodeKind::Table,
            node_type: Some("table".into()),
            properties: IndexMap::new(),
        }
    }

    /// Builder: set kind.
    pub fn with_kind(mut self, kind: LineageNodeKind) -> Self {
        self.kind = kind;
        self.node_type = Some(kind.as_str().into());
        self
    }

    /// Builder: set fqn.
    pub fn with_fqn(mut self, fqn: impl Into<String>) -> Self {
        let f = fqn.into();
        self.fqn = Some(f.clone());
        if self.label.is_empty() {
            self.label = f;
        }
        self
    }
}

/// Kind of table-level lineage edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageEdgeKind {
    /// Generic transform (default).
    Transforms,
    /// Parsed from SQL SELECT / CTAS / VIEW.
    SqlDerived,
    /// Pipeline writes / loads target.
    PipelineProduces,
    /// Dashboard reads source.
    DashboardReads,
    /// Explicit user-declared dependency.
    Declared,
    /// Other free-form (stored as string via Other).
    Other(String),
}

impl LineageEdgeKind {
    /// Serialize to string for graph storage.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Transforms => "transforms",
            Self::SqlDerived => "sql_derived",
            Self::PipelineProduces => "pipeline_produces",
            Self::DashboardReads => "dashboard_reads",
            Self::Declared => "declared",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse from string.
    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "transforms" => Self::Transforms,
            "sql_derived" => Self::SqlDerived,
            "pipeline_produces" => Self::PipelineProduces,
            "dashboard_reads" => Self::DashboardReads,
            "declared" => Self::Declared,
            other => Self::Other(other.into()),
        }
    }
}

/// Directed edge: `from` produces / feeds `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Upstream asset.
    pub from: AssetId,
    /// Downstream asset.
    pub to: AssetId,
    /// Edge kind (string for extensibility; prefer [`LineageEdgeKind`]).
    pub kind: String,
    /// Optional SQL snippet that established the edge.
    #[serde(default)]
    pub sql: Option<String>,
    /// When the edge was observed or declared.
    pub observed_at: UtcTimestamp,
}

impl LineageEdge {
    /// Create a transforms edge.
    pub fn transforms(from: AssetId, to: AssetId) -> Self {
        Self {
            from,
            to,
            kind: LineageEdgeKind::Transforms.as_str().into(),
            sql: None,
            observed_at: UtcTimestamp::now(),
        }
    }

    /// Create an SQL-derived edge.
    pub fn sql_derived(from: AssetId, to: AssetId, sql: impl Into<String>) -> Self {
        Self {
            from,
            to,
            kind: LineageEdgeKind::SqlDerived.as_str().into(),
            sql: Some(sql.into()),
            observed_at: UtcTimestamp::now(),
        }
    }

    /// Attach kind string.
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }
}

/// Column reference within an asset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnRef {
    /// Parent asset.
    pub asset_id: AssetId,
    /// Column name.
    pub column: String,
}

impl ColumnRef {
    /// Construct a column ref.
    pub fn new(asset_id: AssetId, column: impl Into<String>) -> Self {
        Self {
            asset_id,
            column: column.into(),
        }
    }
}

/// Column-level lineage edge (upstream column → downstream column).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnLineageEdge {
    /// Upstream column.
    pub from: ColumnRef,
    /// Downstream column.
    pub to: ColumnRef,
    /// How the mapping was inferred.
    #[serde(default = "default_col_edge_kind")]
    pub kind: String,
    /// Optional expression fragment.
    #[serde(default)]
    pub expression: Option<String>,
    /// Observed at.
    pub observed_at: UtcTimestamp,
}

fn default_col_edge_kind() -> String {
    "maps_to".into()
}

impl ColumnLineageEdge {
    /// Map upstream column into downstream column.
    pub fn maps_to(from: ColumnRef, to: ColumnRef) -> Self {
        Self {
            from,
            to,
            kind: "maps_to".into(),
            expression: None,
            observed_at: UtcTimestamp::now(),
        }
    }

    /// With expression text.
    pub fn with_expression(mut self, expr: impl Into<String>) -> Self {
        self.expression = Some(expr.into());
        self
    }
}

/// Why impact analysis was requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ImpactTrigger {
    /// Table schema / data change.
    TableChange {
        /// Changed asset.
        asset_id: AssetId,
    },
    /// Validation / DQ check failure on an asset.
    ValidationFailed {
        /// Asset whose check failed.
        asset_id: AssetId,
        /// Optional check id.
        #[serde(default)]
        check_id: Option<String>,
        /// Optional message.
        #[serde(default)]
        message: Option<String>,
    },
}

/// One impacted asset in an impact report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactedAsset {
    /// Asset id.
    pub asset_id: AssetId,
    /// Label / FQN.
    pub label: String,
    /// Kind.
    pub kind: LineageNodeKind,
    /// Graph distance from root (1 = direct consumer).
    pub depth: u32,
    /// Path of edge kinds from root (compressed).
    #[serde(default)]
    pub via: Vec<String>,
}

/// Impact report: which datasets, dashboards, and pipelines are affected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    /// Trigger that started the analysis.
    pub trigger: ImpactTrigger,
    /// Root asset under consideration.
    pub root_asset_id: AssetId,
    /// Downstream tables / views.
    pub tables: Vec<ImpactedAsset>,
    /// Downstream datasets.
    pub datasets: Vec<ImpactedAsset>,
    /// Downstream dashboards.
    pub dashboards: Vec<ImpactedAsset>,
    /// Downstream pipelines.
    pub pipelines: Vec<ImpactedAsset>,
    /// Other node kinds.
    pub other: Vec<ImpactedAsset>,
    /// Full downstream subgraph for visualization.
    pub subgraph: Option<LineageSubgraphMeta>,
}

/// Lightweight subgraph meta attached to impact (nodes/edges counts + lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSubgraphMeta {
    /// Nodes.
    pub nodes: Vec<LineageNode>,
    /// Table-level edges.
    pub edges: Vec<LineageEdge>,
    /// Column-level edges in the subgraph (optional filter).
    #[serde(default)]
    pub column_edges: Vec<ColumnLineageEdge>,
}

impl ImpactReport {
    /// Empty report.
    pub fn empty(trigger: ImpactTrigger, root: AssetId) -> Self {
        Self {
            trigger,
            root_asset_id: root,
            tables: vec![],
            datasets: vec![],
            dashboards: vec![],
            pipelines: vec![],
            other: vec![],
            subgraph: None,
        }
    }

    /// Total unique impacted assets (excluding root if not listed).
    pub fn total_affected(&self) -> usize {
        self.tables.len()
            + self.datasets.len()
            + self.dashboards.len()
            + self.pipelines.len()
            + self.other.len()
    }
}
