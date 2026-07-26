export type AssetKind =
  | "table"
  | "view"
  | "materialized_view"
  | "file"
  | "stream"
  | "other";

export interface SourceLocation {
  connector: string;
  uri: string;
  properties?: Record<string, string>;
}

export interface ColumnMeta {
  name: string;
  data_type?: string;
  nullable?: boolean;
  description?: string | null;
}

export interface Asset {
  id: string;
  fqn: string;
  name: string;
  kind: AssetKind;
  description?: string | null;
  location: SourceLocation;
  columns: ColumnMeta[];
  tags?: Record<string, string>;
  health?: string;
  created_at?: string;
  updated_at?: string;
}

export interface ListResponse<T> {
  items: T[];
  count: number;
}

export interface ColumnProfile {
  name: string;
  data_type?: string;
  semantic_type?: string;
  semantic_confidence?: number;
  null_count: number;
  null_percentage?: number;
  distinct_count: number;
  unique_ratio?: number;
  min?: unknown;
  max?: unknown;
  average?: number | null;
  stddev?: number | null;
  histogram?: { label: string; count: number }[];
  stats?: Record<string, unknown>;
}

export interface DatasetProfile {
  run_id: string;
  asset_id: string;
  asset_fqn?: string | null;
  row_count: number;
  columns: ColumnProfile[];
  profiled_at: string;
  profiler?: string | null;
  connector?: string | null;
}

export interface CheckDefinition {
  id: string;
  name: string;
  description?: string | null;
  asset_id: string;
  validator: string;
  severity: string;
  params: Record<string, unknown>;
  enabled: boolean;
  schedule?: string | null;
  created_at?: string;
}

export interface ProposedRule {
  name: string;
  description?: string | null;
  validator: string;
  severity: string;
  params: Record<string, unknown>;
}

export type RuleSuggestionStatus = "pending" | "approved" | "rejected";

export interface RuleSuggestion {
  id: string;
  asset_id: string;
  status: RuleSuggestionStatus;
  proposed: ProposedRule;
  rationale: string;
  confidence: number;
  provider: string;
  model?: string | null;
  profile_run_id?: string | null;
  connector_id?: string | null;
  approved_check_id?: string | null;
  rejection_reason?: string | null;
  reviewed_by?: string | null;
  created_at: string;
  reviewed_at?: string | null;
}

export interface ApproveResult {
  suggestion: RuleSuggestion;
  check: CheckDefinition;
}

export interface AiStatus {
  enabled: boolean;
  default_provider: string;
  providers: PluginInfo[];
}

export interface CheckResult {
  run_id: string;
  check_id: string;
  status: string;
  severity: string;
  message: string;
  metrics?: Record<string, unknown>;
  finished_at: string;
  suite_run_id?: string | null;
}

export interface ValidationRun {
  id: string;
  asset_id?: string | null;
  connector_id: string;
  status: string;
  results: CheckResult[];
  passed: number;
  failed: number;
  warned: number;
  skipped: number;
  errored: number;
  started_at: string;
  finished_at: string;
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  capabilities: string[];
}

export interface LineageNode {
  asset_id: string;
  label: string;
  fqn?: string | null;
  kind?: string;
  node_type?: string | null;
}

export interface LineageEdge {
  from: string;
  to: string;
  kind: string;
  sql?: string | null;
  observed_at?: string;
}

export interface LineageSnapshot {
  nodes: LineageNode[];
  edges: LineageEdge[];
  column_edges?: unknown[];
}

export interface Incident {
  id: string;
  asset_id: string;
  affected_assets: string[];
  source: { type: string; [k: string]: unknown };
  severity: string;
  title: string;
  message: string;
  status: string;
  owner?: string | null;
  field?: string | null;
  detector?: string | null;
  kind?: string | null;
  timeline?: IncidentEvent[];
  notified_channels?: string[];
  created_at: string;
  updated_at: string;
}

export interface IncidentEvent {
  id: string;
  incident_id: string;
  at: string;
  actor?: string | null;
  event_type: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface AnomalyReport {
  run_id: string;
  asset_id: string;
  detector: string;
  findings: {
    detector: string;
    kind?: string;
    field?: string | null;
    message: string;
    severity: string;
    score?: number | null;
  }[];
  incident_ids?: string[];
  finished_at: string;
}
