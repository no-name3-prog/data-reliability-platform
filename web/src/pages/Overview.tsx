import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Activity,
  AlertTriangle,
  Database,
  Plug,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Link } from "react-router-dom";
import {
  Bar,
  BarChart,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { api } from "@/api/client";
import { StatCard } from "@/components/StatCard";
import { Loading } from "@/components/Loading";
import { StatusBadge, SeverityBadge } from "@/components/StatusBadge";
import { formatWhen, shortId } from "@/lib/utils";

export function OverviewPage() {
  const qc = useQueryClient();
  const assets = useQuery({ queryKey: ["assets"], queryFn: () => api.listAssets() });
  const plugins = useQuery({ queryKey: ["plugins"], queryFn: api.listPlugins });
  const incidents = useQuery({
    queryKey: ["incidents"],
    queryFn: () => api.listIncidents(undefined, 100),
  });
  const checks = useQuery({ queryKey: ["checks"], queryFn: () => api.listChecks() });
  const runs = useQuery({
    queryKey: ["validation-runs"],
    queryFn: () => api.listValidationRuns(undefined, 30),
  });

  const seed = useMutation({
    mutationFn: api.discoverMock,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["assets"] });
    },
  });

  if (assets.isLoading || incidents.isLoading) return <Loading />;

  const assetItems = assets.data?.items ?? [];
  const incidentItems = incidents.data?.items ?? [];
  const openIncidents = incidentItems.filter((i) =>
    ["open", "in_progress", "acknowledged", "monitoring"].includes(
      i.status.toLowerCase(),
    ),
  );
  const connectors = (plugins.data?.items ?? []).filter((p) =>
    p.capabilities?.includes("connector"),
  );

  const severityData = ["critical", "high", "medium", "low"].map((s) => ({
    name: s,
    count: incidentItems.filter((i) => i.severity.toLowerCase() === s).length,
  }));

  const kindData = Object.entries(
    assetItems.reduce<Record<string, number>>((acc, a) => {
      acc[a.kind] = (acc[a.kind] || 0) + 1;
      return acc;
    }, {}),
  ).map(([name, value]) => ({ name, value }));

  const passFail = (runs.data?.items ?? []).slice(0, 12).map((r) => ({
    name: shortId(r.id, 6),
    passed: r.passed,
    failed: r.failed,
  }));

  const colors = ["#6366f1", "#22c55e", "#f59e0b", "#f43f5e", "#0ea5e9", "#8b5cf6"];

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight text-slate-900">
            Overview
          </h1>
          <p className="mt-1 text-sm text-slate-500">
            Reliability posture across sources, quality checks, and incidents.
          </p>
        </div>
        <button
          type="button"
          className="btn-primary"
          disabled={seed.isPending}
          onClick={() => seed.mutate()}
        >
          <Sparkles className="h-4 w-4" />
          {seed.isPending ? "Loading dummy data…" : "Load mock data"}
        </button>
      </div>

      {seed.isError ? (
        <div className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          Could not reach API: {(seed.error as Error).message}. Start the stack
          with <code className="font-mono">make up</code>.
        </div>
      ) : null}

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="Datasets"
          value={assets.data?.count ?? 0}
          icon={Database}
          hint={`${kindData.length} kinds`}
        />
        <StatCard
          label="Connectors"
          value={connectors.length}
          icon={Plug}
          hint="Registered plugins"
          tone="good"
        />
        <StatCard
          label="Checks"
          value={checks.data?.count ?? 0}
          icon={ShieldCheck}
          hint="Validation definitions"
        />
        <StatCard
          label="Open incidents"
          value={openIncidents.length}
          icon={AlertTriangle}
          tone={openIncidents.length ? "bad" : "good"}
          hint={`${incidentItems.length} total`}
        />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <div className="card p-5">
          <h2 className="mb-4 text-sm font-semibold text-slate-800">
            Datasets by kind
          </h2>
          {kindData.length ? (
            <div className="h-56">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={kindData}
                    dataKey="value"
                    nameKey="name"
                    innerRadius={50}
                    outerRadius={80}
                    paddingAngle={3}
                  >
                    {kindData.map((_, i) => (
                      <Cell key={i} fill={colors[i % colors.length]} />
                    ))}
                  </Pie>
                  <Tooltip />
                </PieChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <p className="py-10 text-center text-sm text-slate-500">
              No datasets yet — load mock data to begin.
            </p>
          )}
        </div>

        <div className="card p-5">
          <h2 className="mb-4 text-sm font-semibold text-slate-800">
            Incidents by severity
          </h2>
          <div className="h-56">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={severityData}>
                <XAxis dataKey="name" tick={{ fontSize: 12 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
                <Tooltip />
                <Bar dataKey="count" fill="#6366f1" radius={[8, 8, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {passFail.length ? (
        <div className="card p-5">
          <div className="mb-4 flex items-center gap-2">
            <Activity className="h-4 w-4 text-brand-600" />
            <h2 className="text-sm font-semibold text-slate-800">
              Recent validation suites
            </h2>
          </div>
          <div className="h-52">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={passFail}>
                <XAxis dataKey="name" tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
                <Tooltip />
                <Bar dataKey="passed" stackId="a" fill="#22c55e" />
                <Bar dataKey="failed" stackId="a" fill="#f43f5e" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <div className="card overflow-hidden">
          <div className="flex items-center justify-between border-b border-slate-100 px-5 py-3">
            <h2 className="text-sm font-semibold">Recent incidents</h2>
            <Link to="/incidents" className="text-xs font-medium text-brand-600">
              View all
            </Link>
          </div>
          <ul className="divide-y divide-slate-100">
            {incidentItems.slice(0, 5).map((i) => (
              <li key={i.id}>
                <Link
                  to={`/incidents/${i.id}`}
                  className="flex flex-col gap-1 px-5 py-3 hover:bg-slate-50 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-slate-800">
                      {i.title}
                    </p>
                    <p className="text-xs text-slate-500">{formatWhen(i.created_at)}</p>
                  </div>
                  <div className="flex gap-2">
                    <SeverityBadge severity={i.severity} />
                    <StatusBadge status={i.status} />
                  </div>
                </Link>
              </li>
            ))}
            {!incidentItems.length ? (
              <li className="px-5 py-8 text-center text-sm text-slate-500">
                No incidents yet
              </li>
            ) : null}
          </ul>
        </div>

        <div className="card overflow-hidden">
          <div className="flex items-center justify-between border-b border-slate-100 px-5 py-3">
            <h2 className="text-sm font-semibold">Datasets</h2>
            <Link to="/datasets" className="text-xs font-medium text-brand-600">
              Browse
            </Link>
          </div>
          <ul className="divide-y divide-slate-100">
            {assetItems.slice(0, 6).map((a) => (
              <li key={a.id}>
                <Link
                  to={`/datasets/${a.id}`}
                  className="flex items-center justify-between gap-3 px-5 py-3 hover:bg-slate-50"
                >
                  <div className="min-w-0">
                    <p className="truncate font-medium text-slate-800">{a.name}</p>
                    <p className="truncate font-mono text-xs text-slate-500">
                      {a.fqn}
                    </p>
                  </div>
                  <span className="badge bg-slate-100 text-slate-600 capitalize">
                    {a.kind}
                  </span>
                </Link>
              </li>
            ))}
            {!assetItems.length ? (
              <li className="px-5 py-8 text-center text-sm text-slate-500">
                <button
                  type="button"
                  className="btn-secondary"
                  onClick={() => seed.mutate()}
                >
                  <RefreshCw className="h-4 w-4" /> Load mock datasets
                </button>
              </li>
            ) : null}
          </ul>
        </div>
      </div>
    </div>
  );
}
