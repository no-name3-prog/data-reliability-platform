import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import { Loading, EmptyState } from "@/components/Loading";
import { SearchInput } from "@/components/SearchInput";
import { StatusBadge } from "@/components/StatusBadge";
import { filterByQuery, formatWhen, shortId } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";

export function ValidationPage() {
  const [q, setQ] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [assetFilter, setAssetFilter] = useState("all");
  const [suggestAsset, setSuggestAsset] = useState("");
  const [suggestionStatus, setSuggestionStatus] = useState("pending");
  const dq = useDebounced(q);
  const qc = useQueryClient();

  const checks = useQuery({ queryKey: ["checks"], queryFn: () => api.listChecks() });
  const runs = useQuery({
    queryKey: ["validation-runs"],
    queryFn: () => api.listValidationRuns(undefined, 50),
  });
  const assets = useQuery({ queryKey: ["assets"], queryFn: () => api.listAssets() });
  const rules = useQuery({ queryKey: ["validation-rules"], queryFn: api.listValidationRules });
  const aiStatus = useQuery({ queryKey: ["ai-status"], queryFn: api.aiStatus });
  const suggestions = useQuery({
    queryKey: ["ai-suggestions", suggestionStatus],
    queryFn: () =>
      api.listSuggestions({
        status: suggestionStatus === "all" ? undefined : suggestionStatus,
        limit: 50,
      }),
  });

  const runCheck = useMutation({
    mutationFn: (id: string) => api.runCheck(id, "mock"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["validation-runs"] });
      qc.invalidateQueries({ queryKey: ["incidents"] });
    },
  });

  const createDemo = useMutation({
    mutationFn: async () => {
      const orders = assets.data?.items.find((a) => a.name === "orders");
      if (!orders) throw new Error("Load mock datasets first (orders missing)");
      const check = await api.createCheck({
        name: "orders email not null",
        asset_id: orders.id,
        validator: "not_null",
        params: { column: "customer_email" },
      });
      return api.runCheck(check.id, "mock");
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["checks"] });
      qc.invalidateQueries({ queryKey: ["validation-runs"] });
      qc.invalidateQueries({ queryKey: ["incidents"] });
    },
  });

  const suggest = useMutation({
    mutationFn: async () => {
      const id =
        suggestAsset ||
        assets.data?.items.find((a) => a.name === "orders")?.id ||
        assets.data?.items[0]?.id;
      if (!id) throw new Error("Discover datasets first, then pick an asset");
      // Prefer a profile when available for better suggestions
      try {
        await api.runProfile(id, "mock");
      } catch {
        /* optional */
      }
      return api.suggestRules(id, {
        connector: "mock",
        provider: aiStatus.data?.default_provider,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ai-suggestions"] });
      setSuggestionStatus("pending");
    },
  });

  const approve = useMutation({
    mutationFn: (id: string) => api.approveSuggestion(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ai-suggestions"] });
      qc.invalidateQueries({ queryKey: ["checks"] });
    },
  });

  const reject = useMutation({
    mutationFn: (id: string) => api.rejectSuggestion(id, "Not useful"),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["ai-suggestions"] });
    },
  });

  const assetName = useMemo(() => {
    const m = new Map((assets.data?.items ?? []).map((a) => [a.id, a.name]));
    return (id: string) => m.get(id) || shortId(id, 10);
  }, [assets.data]);

  const filteredChecks = useMemo(() => {
    let list = checks.data?.items ?? [];
    if (assetFilter !== "all") list = list.filter((c) => c.asset_id === assetFilter);
    return filterByQuery(list, dq, (c) => [c.name, c.validator, c.id]);
  }, [checks.data, assetFilter, dq]);

  const filteredRuns = useMemo(() => {
    let list = runs.data?.items ?? [];
    if (statusFilter !== "all")
      list = list.filter((r) => r.status.toLowerCase() === statusFilter);
    return list;
  }, [runs.data, statusFilter]);

  if (checks.isLoading) return <Loading />;

  const pendingCount =
    suggestions.data?.items.filter((s) => s.status === "pending").length ?? 0;

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Validation</h1>
          <p className="mt-1 text-sm text-slate-500">
            Check definitions, suite runs, and AI-suggested rules (review before
            they go live).
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="btn-secondary"
            disabled={suggest.isPending || !aiStatus.data?.enabled}
            onClick={() => suggest.mutate()}
          >
            {suggest.isPending ? "Suggesting…" : "Suggest rules (AI)"}
          </button>
          <button
            type="button"
            className="btn-primary"
            disabled={createDemo.isPending}
            onClick={() => createDemo.mutate()}
          >
            Run demo not-null check
          </button>
        </div>
      </div>

      {createDemo.isError || suggest.isError ? (
        <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
          {(createDemo.error as Error | null)?.message ||
            (suggest.error as Error | null)?.message}
        </div>
      ) : null}

      {/* AI suggestions review queue */}
      <div className="card overflow-hidden">
        <div className="flex flex-col gap-3 border-b border-slate-100 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h2 className="text-sm font-semibold">
              AI rule suggestions
              {pendingCount ? (
                <span className="ml-2 rounded-full bg-indigo-100 px-2 py-0.5 text-xs font-medium text-indigo-700">
                  {pendingCount} pending
                </span>
              ) : null}
            </h2>
            <p className="mt-0.5 text-xs text-slate-500">
              {aiStatus.data?.enabled === false
                ? "AI layer is disabled in config."
                : `Provider: ${aiStatus.data?.default_provider ?? "…"} · Suggestions stay inactive until you approve.`}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <select
              className="input max-w-[180px]"
              value={suggestAsset}
              onChange={(e) => setSuggestAsset(e.target.value)}
            >
              <option value="">Auto (orders / first)</option>
              {(assets.data?.items ?? []).map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
            <select
              className="input max-w-[140px]"
              value={suggestionStatus}
              onChange={(e) => setSuggestionStatus(e.target.value)}
            >
              <option value="pending">Pending</option>
              <option value="approved">Approved</option>
              <option value="rejected">Rejected</option>
              <option value="all">All</option>
            </select>
          </div>
        </div>
        {!suggestions.data?.items.length ? (
          <div className="p-6">
            <EmptyState
              title="No suggestions yet"
              description="Load mock datasets, then click “Suggest rules (AI)” to propose checks from schema and profiling."
            />
          </div>
        ) : (
          <ul className="divide-y divide-slate-100">
            {suggestions.data.items.map((s) => (
              <li
                key={s.id}
                className="flex flex-col gap-3 px-4 py-3 lg:flex-row lg:items-start lg:justify-between"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-slate-800">{s.proposed.name}</p>
                    <StatusBadge status={s.status} />
                    <span className="text-xs text-slate-400">
                      conf {(s.confidence * 100).toFixed(0)}% · {s.provider}
                    </span>
                  </div>
                  <p className="mt-1 text-xs text-slate-500">
                    {s.proposed.validator} · {assetName(s.asset_id)} ·{" "}
                    <SeverityBadgeLite severity={s.proposed.severity} />
                    {s.proposed.params?.column
                      ? ` · col ${String(s.proposed.params.column)}`
                      : ""}
                  </p>
                  {s.rationale ? (
                    <p className="mt-1 text-sm text-slate-600">{s.rationale}</p>
                  ) : null}
                  {s.approved_check_id ? (
                    <p className="mt-1 font-mono text-xs text-emerald-700">
                      check {shortId(s.approved_check_id, 12)}
                    </p>
                  ) : null}
                </div>
                {s.status === "pending" ? (
                  <div className="flex shrink-0 gap-2">
                    <button
                      type="button"
                      className="btn-primary"
                      disabled={approve.isPending}
                      onClick={() => approve.mutate(s.id)}
                    >
                      Approve
                    </button>
                    <button
                      type="button"
                      className="btn-secondary"
                      disabled={reject.isPending}
                      onClick={() => reject.mutate(s.id)}
                    >
                      Reject
                    </button>
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="flex flex-col gap-3 lg:flex-row">
        <SearchInput value={q} onChange={setQ} placeholder="Search checks…" />
        <select
          className="input lg:max-w-[200px]"
          value={assetFilter}
          onChange={(e) => setAssetFilter(e.target.value)}
        >
          <option value="all">All assets</option>
          {(assets.data?.items ?? []).map((a) => (
            <option key={a.id} value={a.id}>
              {a.name}
            </option>
          ))}
        </select>
        <select
          className="input lg:max-w-[160px]"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
        >
          <option value="all">All suite statuses</option>
          <option value="passed">passed</option>
          <option value="failed">failed</option>
          <option value="warned">warned</option>
        </select>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <div className="card overflow-hidden">
          <div className="border-b border-slate-100 px-4 py-3 text-sm font-semibold">
            Checks ({filteredChecks.length})
          </div>
          {!filteredChecks.length ? (
            <div className="p-6">
              <EmptyState
                title="No checks yet"
                description="Approve an AI suggestion or create a not-null check on mock orders email."
              />
            </div>
          ) : (
            <ul className="divide-y divide-slate-100">
              {filteredChecks.map((c) => (
                <li
                  key={c.id}
                  className="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="min-w-0">
                    <p className="font-medium text-slate-800">{c.name}</p>
                    <p className="text-xs text-slate-500">
                      {c.validator} · {assetName(c.asset_id)} ·{" "}
                      <SeverityBadgeLite severity={c.severity} />
                    </p>
                  </div>
                  <button
                    type="button"
                    className="btn-secondary shrink-0"
                    disabled={runCheck.isPending}
                    onClick={() => runCheck.mutate(c.id)}
                  >
                    Run
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="card overflow-hidden">
          <div className="border-b border-slate-100 px-4 py-3 text-sm font-semibold">
            Suite runs
          </div>
          <ul className="divide-y divide-slate-100">
            {filteredRuns.map((r) => (
              <li key={r.id} className="px-4 py-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-mono text-xs text-slate-500">
                    {shortId(r.id, 12)}
                  </span>
                  <StatusBadge status={r.status} />
                </div>
                <p className="mt-1 text-sm text-slate-700">
                  pass {r.passed} · fail {r.failed} · warn {r.warned}
                </p>
                <p className="text-xs text-slate-400">{formatWhen(r.finished_at)}</p>
              </li>
            ))}
            {!filteredRuns.length ? (
              <li className="px-4 py-8 text-center text-sm text-slate-500">
                No suite runs yet
              </li>
            ) : null}
          </ul>
        </div>
      </div>

      <div className="card p-4">
        <h2 className="text-sm font-semibold">Available rule plugins</h2>
        <div className="mt-3 flex flex-wrap gap-2">
          {(rules.data?.items ?? []).map((r) => (
            <span key={r.id} className="badge bg-slate-100 text-slate-700">
              {r.id}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

function SeverityBadgeLite({ severity }: { severity: string }) {
  return <span className="capitalize">{severity}</span>;
}
