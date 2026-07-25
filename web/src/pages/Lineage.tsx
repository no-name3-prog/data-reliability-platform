import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api } from "@/api/client";
import { LineageGraph } from "@/components/LineageGraph";
import { Loading, EmptyState } from "@/components/Loading";
import { SearchInput } from "@/components/SearchInput";
import { filterByQuery, shortId } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";

const SAMPLE_SQL = `CREATE VIEW analytics.orders_enriched AS
SELECT o.order_id, o.amount, u.email AS customer_email
FROM mock.public.orders o
JOIN mock.public.users u ON true`;

export function LineagePage() {
  const [params, setParams] = useSearchParams();
  const focus = params.get("focus") || "";
  const [q, setQ] = useState("");
  const [sql, setSql] = useState(SAMPLE_SQL);
  const dq = useDebounced(q);
  const qc = useQueryClient();

  const lineage = useQuery({ queryKey: ["lineage"], queryFn: api.lineage });
  const impact = useQuery({
    queryKey: ["impact", focus],
    queryFn: () => api.impact(focus),
    enabled: !!focus,
  });

  const parse = useMutation({
    mutationFn: () => api.parseSql(sql),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["lineage"] }),
  });

  const nodes = useMemo(() => {
    const list = lineage.data?.nodes ?? [];
    return filterByQuery(list, dq, (n) => [n.label, n.fqn, n.asset_id, n.kind]);
  }, [lineage.data, dq]);

  const edges = useMemo(() => {
    const ids = new Set(nodes.map((n) => n.asset_id));
    return (lineage.data?.edges ?? []).filter(
      (e) => ids.has(e.from) && ids.has(e.to),
    );
  }, [lineage.data, nodes]);

  if (lineage.isLoading) return <Loading />;

  const impactObj = impact.data as
    | {
        tables?: { label: string }[];
        datasets?: { label: string }[];
        dashboards?: { label: string }[];
        pipelines?: { label: string }[];
      }
    | undefined;

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Lineage</h1>
        <p className="mt-1 text-sm text-slate-500">
          Table-level dependency graph, SQL ingest, and impact analysis.
        </p>
      </div>

      <div className="grid gap-4 lg:grid-cols-[1fr_320px]">
        <div className="space-y-4">
          <div className="card p-4">
            <div className="mb-3 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <SearchInput
                value={q}
                onChange={setQ}
                placeholder="Filter nodes…"
              />
              <span className="text-xs text-slate-500">
                {nodes.length} nodes · {edges.length} edges
              </span>
            </div>
            {!nodes.length ? (
              <EmptyState
                title="Lineage graph is empty"
                description="Parse sample SQL to create table and column edges."
              />
            ) : (
              <LineageGraph
                nodes={nodes}
                edges={edges}
                highlightId={focus || undefined}
                onSelect={(id) => setParams({ focus: id })}
              />
            )}
          </div>

          <div className="card p-4">
            <h2 className="text-sm font-semibold">Parse SQL into lineage</h2>
            <textarea
              className="input mt-3 min-h-[120px] font-mono text-xs"
              value={sql}
              onChange={(e) => setSql(e.target.value)}
            />
            <div className="mt-3 flex flex-wrap gap-2">
              <button
                type="button"
                className="btn-primary"
                disabled={parse.isPending}
                onClick={() => parse.mutate()}
              >
                Ingest SQL
              </button>
              <button
                type="button"
                className="btn-secondary"
                onClick={() => setSql(SAMPLE_SQL)}
              >
                Reset sample
              </button>
            </div>
            {parse.isError ? (
              <p className="mt-2 text-sm text-rose-600">
                {(parse.error as Error).message}
              </p>
            ) : null}
            {parse.isSuccess ? (
              <pre className="mt-3 overflow-auto rounded-xl bg-slate-50 p-3 text-xs">
                {JSON.stringify(parse.data, null, 2)}
              </pre>
            ) : null}
          </div>
        </div>

        <aside className="card p-4">
          <h2 className="text-sm font-semibold">Impact panel</h2>
          {!focus ? (
            <p className="mt-3 text-sm text-slate-500">
              Click a node to compute downstream impact.
            </p>
          ) : impact.isLoading ? (
            <Loading label="Analyzing…" />
          ) : (
            <div className="mt-3 space-y-3 text-sm">
              <p className="font-mono text-xs text-slate-500">
                root {shortId(focus, 14)}
              </p>
              <ImpactList title="Tables" items={impactObj?.tables} />
              <ImpactList title="Datasets" items={impactObj?.datasets} />
              <ImpactList title="Dashboards" items={impactObj?.dashboards} />
              <ImpactList title="Pipelines" items={impactObj?.pipelines} />
            </div>
          )}

          <div className="mt-6 border-t border-slate-100 pt-4">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              Edges
            </h3>
            <ul className="mt-2 max-h-64 space-y-1 overflow-y-auto text-xs">
              {(lineage.data?.edges ?? []).slice(0, 40).map((e, i) => (
                <li key={i} className="rounded-lg bg-slate-50 px-2 py-1.5">
                  <span className="font-mono">{shortId(e.from, 6)}</span>
                  <span className="text-slate-400"> → </span>
                  <span className="font-mono">{shortId(e.to, 6)}</span>
                  <span className="ml-1 text-slate-400">({e.kind})</span>
                </li>
              ))}
            </ul>
          </div>
        </aside>
      </div>
    </div>
  );
}

function ImpactList({
  title,
  items,
}: {
  title: string;
  items?: { label: string }[];
}) {
  return (
    <div>
      <h3 className="text-xs font-semibold uppercase tracking-wide text-slate-500">
        {title} ({items?.length ?? 0})
      </h3>
      <ul className="mt-1 space-y-0.5">
        {(items ?? []).slice(0, 8).map((x, i) => (
          <li key={i} className="truncate text-slate-700">
            {x.label}
          </li>
        ))}
        {!items?.length ? (
          <li className="text-slate-400">None</li>
        ) : null}
      </ul>
    </div>
  );
}
