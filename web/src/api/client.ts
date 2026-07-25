import type {
  AnomalyReport,
  Asset,
  CheckDefinition,
  CheckResult,
  DatasetProfile,
  Incident,
  IncidentEvent,
  LineageSnapshot,
  ListResponse,
  PluginInfo,
  ValidationRun,
} from "./types";

const BASE = "";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers || {}),
    },
  });
  if (!res.ok) {
    let detail = res.statusText;
    try {
      const body = await res.json();
      detail = body.error || body.message || JSON.stringify(body);
    } catch {
      /* ignore */
    }
    throw new Error(`${res.status}: ${detail}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const api = {
  health: () =>
    fetch(`${BASE}/readyz`)
      .then((r) => r.ok)
      .catch(() => false),

  listAssets: (limit = 200) =>
    request<ListResponse<Asset>>(`/v1/assets?limit=${limit}`),

  getAsset: (id: string) => request<Asset>(`/v1/assets/${id}`),

  discoverMock: () =>
    request<ListResponse<Asset>>(`/v1/assets/discover`, {
      method: "POST",
      body: JSON.stringify({ connector: "mock", uri: "mock://local" }),
    }),

  listPlugins: () => request<ListResponse<PluginInfo>>(`/v1/plugins`),

  getProfile: (assetId: string) =>
    request<DatasetProfile | null>(`/v1/assets/${assetId}/profile`),

  listProfiles: (assetId: string, limit = 20) =>
    request<ListResponse<DatasetProfile>>(
      `/v1/assets/${assetId}/profiles?limit=${limit}`,
    ),

  runProfile: (assetId: string, connector = "mock") =>
    request<DatasetProfile>(`/v1/assets/${assetId}/profile`, {
      method: "POST",
      body: JSON.stringify({ connector }),
    }),

  listChecks: (assetId?: string) => {
    const q = assetId ? `?asset_id=${encodeURIComponent(assetId)}` : "";
    return request<ListResponse<CheckDefinition>>(`/v1/checks${q}`);
  },

  createCheck: (body: {
    name: string;
    asset_id: string;
    validator: string;
    params?: Record<string, unknown>;
  }) =>
    request<CheckDefinition>(`/v1/checks`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  runCheck: (id: string, connector = "mock") =>
    request<CheckResult>(`/v1/checks/${id}/run`, {
      method: "POST",
      body: JSON.stringify({ connector }),
    }),

  listCheckResults: (id: string, limit = 20) =>
    request<ListResponse<CheckResult>>(
      `/v1/checks/${id}/results?limit=${limit}`,
    ),

  listValidationRules: () =>
    request<ListResponse<PluginInfo>>(`/v1/validation/rules`),

  listValidationRuns: (assetId?: string, limit = 50) => {
    const params = new URLSearchParams();
    if (assetId) params.set("asset_id", assetId);
    params.set("limit", String(limit));
    return request<ListResponse<ValidationRun>>(
      `/v1/validation/runs?${params}`,
    );
  },

  validateAsset: (assetId: string, connector = "mock") =>
    request<ValidationRun>(`/v1/assets/${assetId}/validate`, {
      method: "POST",
      body: JSON.stringify({ connector }),
    }),

  lineage: () => request<LineageSnapshot>(`/v1/lineage`),

  lineageUpstream: (id: string, depth = 10) =>
    request<LineageSnapshot>(
      `/v1/lineage/assets/${id}/upstream?depth=${depth}`,
    ),

  lineageDownstream: (id: string, depth = 10) =>
    request<LineageSnapshot>(
      `/v1/lineage/assets/${id}/downstream?depth=${depth}`,
    ),

  parseSql: (sql: string) =>
    request<unknown>(`/v1/lineage/parse-sql`, {
      method: "POST",
      body: JSON.stringify({ sql }),
    }),

  impact: (id: string) =>
    request<unknown>(`/v1/lineage/assets/${id}/impact`),

  listIncidents: (assetId?: string, limit = 100) => {
    const params = new URLSearchParams({ limit: String(limit) });
    if (assetId) params.set("asset_id", assetId);
    return request<ListResponse<Incident>>(`/v1/incidents?${params}`);
  },

  getIncident: (id: string) => request<Incident>(`/v1/incidents/${id}`),

  incidentHistory: (id: string) =>
    request<ListResponse<IncidentEvent>>(`/v1/incidents/${id}/history`),

  setIncidentStatus: (id: string, status: string, note?: string) =>
    request<Incident>(`/v1/incidents/${id}/status`, {
      method: "POST",
      body: JSON.stringify({ status, note, actor: "dashboard" }),
    }),

  assignOwner: (id: string, owner: string) =>
    request<Incident>(`/v1/incidents/${id}/owner`, {
      method: "POST",
      body: JSON.stringify({ owner, actor: "dashboard" }),
    }),

  analyzeAnomalies: (assetId: string) =>
    request<AnomalyReport>(`/v1/assets/${assetId}/anomalies/analyze`, {
      method: "POST",
    }),
};
