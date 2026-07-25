import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import {
  Activity,
  ArrowLeft,
  GitBranch,
  Play,
  ShieldCheck,
} from "lucide-react";
import { api } from "@/api/client";
import { Loading, EmptyState } from "@/components/Loading";
import { formatWhen, shortId } from "@/lib/utils";
import {
  Bar,
  BarChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

export function DatasetDetailPage() {
  const { id = "" } = useParams();
  const qc = useQueryClient();
  const asset = useQuery({
    queryKey: ["asset", id],
    queryFn: () => api.getAsset(id),
    enabled: !!id,
  });
  const profile = useQuery({
    queryKey: ["profile", id],
    queryFn: () => api.getProfile(id),
    enabled: !!id,
  });
  const runProfile = useMutation({
    mutationFn: () => api.runProfile(id, asset.data?.location?.connector || "mock"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["profile", id] });
      qc.invalidateQueries({ queryKey: ["profiles", id] });
    },
  });
  const validate = useMutation({
    mutationFn: () =>
      api.validateAsset(id, asset.data?.location?.connector || "mock"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["validation-runs"] });
      qc.invalidateQueries({ queryKey: ["incidents"] });
    },
  });

  if (asset.isLoading) return <Loading />;
  if (asset.isError || !asset.data)
    return (
      <EmptyState
        title="Dataset not found"
        description={(asset.error as Error)?.message}
        action={
          <Link to="/datasets" className="btn-secondary">
            Back to datasets
          </Link>
        }
      />
    );

  const a = asset.data;
  const p = profile.data;
  const nullChart =
    p?.columns?.map((c) => ({
      name: c.name,
      null_pct: Number(c.null_percentage?.toFixed?.(1) ?? c.null_percentage ?? 0),
    })) ?? [];

  return (
    <div className="space-y-5">
      <Link
        to="/datasets"
        className="inline-flex items-center gap-1 text-sm text-slate-500 hover:text-slate-800"
      >
        <ArrowLeft className="h-4 w-4" /> Datasets
      </Link>

      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">{a.name}</h1>
          <p className="mt-1 font-mono text-sm text-slate-500">{a.fqn}</p>
          <div className="mt-2 flex flex-wrap gap-2 text-xs text-slate-500">
            <span className="badge bg-slate-100 capitalize">{a.kind}</span>
            <span className="badge bg-slate-100">{a.location.connector}</span>
            <span className="font-mono">{shortId(a.id, 14)}</span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="btn-secondary"
            disabled={runProfile.isPending}
            onClick={() => runProfile.mutate()}
          >
            <Activity className="h-4 w-4" /> Profile
          </button>
          <button
            type="button"
            className="btn-secondary"
            disabled={validate.isPending}
            onClick={() => validate.mutate()}
          >
            <ShieldCheck className="h-4 w-4" /> Validate
          </button>
          <Link to={`/lineage?focus=${a.id}`} className="btn-secondary">
            <GitBranch className="h-4 w-4" /> Lineage
          </Link>
          <Link to={`/profiling?asset=${a.id}`} className="btn-primary">
            <Play className="h-4 w-4" /> Open profiling
          </Link>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="card p-4 lg:col-span-1">
          <h2 className="text-sm font-semibold">Source</h2>
          <dl className="mt-3 space-y-2 text-sm">
            <div>
              <dt className="text-xs text-slate-500">URI</dt>
              <dd className="break-all font-mono text-xs">{a.location.uri}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500">Updated</dt>
              <dd>{formatWhen(a.updated_at)}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500">Columns (catalog)</dt>
              <dd>{a.columns?.length ?? 0}</dd>
            </div>
          </dl>
        </div>

        <div className="card p-4 lg:col-span-2">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold">Latest profile</h2>
            {p ? (
              <span className="text-xs text-slate-500">
                {p.row_count} rows · {formatWhen(p.profiled_at)}
              </span>
            ) : null}
          </div>
          {!p ? (
            <p className="mt-6 text-center text-sm text-slate-500">
              No profile yet. Click Profile to analyze mock sample rows.
            </p>
          ) : (
            <div className="mt-4 h-52">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={nullChart}>
                  <XAxis dataKey="name" tick={{ fontSize: 11 }} />
                  <YAxis tick={{ fontSize: 11 }} unit="%" />
                  <Tooltip />
                  <Bar dataKey="null_pct" fill="#6366f1" radius={[6, 6, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}
        </div>
      </div>

      <div className="card overflow-hidden">
        <div className="border-b border-slate-100 px-4 py-3 text-sm font-semibold">
          Schema
        </div>
        <div className="overflow-x-auto">
          <table className="min-w-full text-left text-sm">
            <thead className="bg-slate-50 text-xs uppercase text-slate-500">
              <tr>
                <th className="px-4 py-2">Column</th>
                <th className="px-4 py-2">Type</th>
                <th className="px-4 py-2">Null %</th>
                <th className="px-4 py-2">Distinct</th>
                <th className="px-4 py-2">Semantic</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {(p?.columns?.length ? p.columns : a.columns || []).map((c) => {
                const col = c as {
                  name: string;
                  data_type?: string;
                  null_percentage?: number;
                  distinct_count?: number;
                  semantic_type?: string;
                };
                return (
                  <tr key={col.name}>
                    <td className="px-4 py-2 font-medium">{col.name}</td>
                    <td className="px-4 py-2 font-mono text-xs">
                      {col.data_type || "—"}
                    </td>
                    <td className="px-4 py-2">
                      {col.null_percentage != null
                        ? `${Number(col.null_percentage).toFixed(1)}%`
                        : "—"}
                    </td>
                    <td className="px-4 py-2">{col.distinct_count ?? "—"}</td>
                    <td className="px-4 py-2 capitalize">
                      {col.semantic_type || "—"}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
