import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { api } from "@/api/client";
import { Loading, EmptyState } from "@/components/Loading";
import { SearchInput } from "@/components/SearchInput";
import { formatWhen, filterByQuery } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";
import {
  Bar,
  BarChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

export function ProfilingPage() {
  const [params, setParams] = useSearchParams();
  const selected = params.get("asset") || "";
  const [q, setQ] = useState("");
  const dq = useDebounced(q);
  const qc = useQueryClient();

  const assets = useQuery({ queryKey: ["assets"], queryFn: () => api.listAssets() });
  const filtered = useMemo(
    () =>
      filterByQuery(assets.data?.items ?? [], dq, (a) => [a.name, a.fqn]),
    [assets.data, dq],
  );

  const profile = useQuery({
    queryKey: ["profile", selected],
    queryFn: () => api.getProfile(selected),
    enabled: !!selected,
  });
  const history = useQuery({
    queryKey: ["profiles", selected],
    queryFn: () => api.listProfiles(selected, 15),
    enabled: !!selected,
  });

  const run = useMutation({
    mutationFn: () => {
      const a = assets.data?.items.find((x) => x.id === selected);
      return api.runProfile(selected, a?.location?.connector || "mock");
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["profile", selected] });
      qc.invalidateQueries({ queryKey: ["profiles", selected] });
    },
  });

  if (assets.isLoading) return <Loading />;

  const p = profile.data;
  const chart =
    p?.columns?.map((c) => ({
      name: c.name,
      nulls: c.null_count,
      distinct: c.distinct_count,
    })) ?? [];

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Profiling</h1>
        <p className="mt-1 text-sm text-slate-500">
          Browse statistical profiles, null rates, and history.
        </p>
      </div>

      <div className="grid gap-4 lg:grid-cols-[280px_1fr]">
        <div className="card flex max-h-[70vh] flex-col overflow-hidden p-3">
          <SearchInput value={q} onChange={setQ} placeholder="Filter datasets…" />
          <ul className="mt-3 flex-1 space-y-0.5 overflow-y-auto">
            {filtered.map((a) => (
              <li key={a.id}>
                <button
                  type="button"
                  onClick={() => setParams({ asset: a.id })}
                  className={`w-full rounded-xl px-3 py-2 text-left text-sm transition ${
                    selected === a.id
                      ? "bg-brand-50 text-brand-800"
                      : "hover:bg-slate-50"
                  }`}
                >
                  <div className="font-medium">{a.name}</div>
                  <div className="truncate font-mono text-[11px] text-slate-400">
                    {a.fqn}
                  </div>
                </button>
              </li>
            ))}
          </ul>
        </div>

        <div className="space-y-4">
          {!selected ? (
            <EmptyState
              title="Select a dataset"
              description="Choose an asset to view profiling results."
            />
          ) : profile.isLoading ? (
            <Loading />
          ) : !p ? (
            <EmptyState
              title="No profile yet"
              description="Run the profiler on mock sample rows."
              action={
                <button type="button" className="btn-primary" onClick={() => run.mutate()}>
                  Run profile
                </button>
              }
            />
          ) : (
            <>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h2 className="text-lg font-semibold">
                    {assets.data?.items.find((a) => a.id === selected)?.name}
                  </h2>
                  <p className="text-sm text-slate-500">
                    {p.row_count} rows · profiled {formatWhen(p.profiled_at)}
                  </p>
                </div>
                <div className="flex gap-2">
                  <Link to={`/datasets/${selected}`} className="btn-secondary">
                    Dataset
                  </Link>
                  <button
                    type="button"
                    className="btn-primary"
                    disabled={run.isPending}
                    onClick={() => run.mutate()}
                  >
                    Re-run profile
                  </button>
                </div>
              </div>

              <div className="card p-4">
                <h3 className="mb-3 text-sm font-semibold">Nulls & distinct</h3>
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={chart}>
                      <XAxis dataKey="name" tick={{ fontSize: 11 }} />
                      <YAxis tick={{ fontSize: 11 }} />
                      <Tooltip />
                      <Bar dataKey="nulls" fill="#f43f5e" radius={[4, 4, 0, 0]} />
                      <Bar dataKey="distinct" fill="#6366f1" radius={[4, 4, 0, 0]} />
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              </div>

              <div className="card overflow-hidden">
                <div className="border-b border-slate-100 px-4 py-3 text-sm font-semibold">
                  Column statistics
                </div>
                <div className="overflow-x-auto">
                  <table className="min-w-full text-left text-sm">
                    <thead className="bg-slate-50 text-xs uppercase text-slate-500">
                      <tr>
                        <th className="px-4 py-2">Column</th>
                        <th className="px-4 py-2">Null %</th>
                        <th className="px-4 py-2">Unique ratio</th>
                        <th className="px-4 py-2">Avg</th>
                        <th className="px-4 py-2">Semantic</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-100">
                      {p.columns.map((c) => (
                        <tr key={c.name}>
                          <td className="px-4 py-2 font-medium">{c.name}</td>
                          <td className="px-4 py-2">
                            {c.null_percentage != null
                              ? `${Number(c.null_percentage).toFixed(1)}%`
                              : "—"}
                          </td>
                          <td className="px-4 py-2">
                            {c.unique_ratio != null
                              ? Number(c.unique_ratio).toFixed(2)
                              : "—"}
                          </td>
                          <td className="px-4 py-2">
                            {c.average != null ? Number(c.average).toFixed(2) : "—"}
                          </td>
                          <td className="px-4 py-2 capitalize">
                            {c.semantic_type || "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>

              <div className="card p-4">
                <h3 className="mb-2 text-sm font-semibold">Profile history</h3>
                <ul className="space-y-2 text-sm">
                  {(history.data?.items ?? []).map((h) => (
                    <li
                      key={h.run_id}
                      className="flex justify-between rounded-lg bg-slate-50 px-3 py-2"
                    >
                      <span className="font-mono text-xs">{h.run_id}</span>
                      <span className="text-slate-500">
                        {h.row_count} rows · {formatWhen(h.profiled_at)}
                      </span>
                    </li>
                  ))}
                  {!history.data?.items?.length ? (
                    <li className="text-slate-500">No history yet</li>
                  ) : null}
                </ul>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
