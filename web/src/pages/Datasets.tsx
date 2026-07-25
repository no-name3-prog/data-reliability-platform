import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { Sparkles } from "lucide-react";
import { api } from "@/api/client";
import { SearchInput } from "@/components/SearchInput";
import { EmptyState, Loading } from "@/components/Loading";
import { filterByQuery, formatWhen, shortId } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";

export function DatasetsPage() {
  const [q, setQ] = useState("");
  const [kind, setKind] = useState("all");
  const [connector, setConnector] = useState("all");
  const dq = useDebounced(q);
  const qc = useQueryClient();
  const assets = useQuery({ queryKey: ["assets"], queryFn: () => api.listAssets() });
  const seed = useMutation({
    mutationFn: api.discoverMock,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["assets"] }),
  });

  const kinds = useMemo(() => {
    const s = new Set((assets.data?.items ?? []).map((a) => a.kind));
    return ["all", ...Array.from(s).sort()];
  }, [assets.data]);

  const connectors = useMemo(() => {
    const s = new Set(
      (assets.data?.items ?? []).map((a) => a.location?.connector).filter(Boolean),
    );
    return ["all", ...Array.from(s).sort()];
  }, [assets.data]);

  const items = useMemo(() => {
    let list = assets.data?.items ?? [];
    if (kind !== "all") list = list.filter((a) => a.kind === kind);
    if (connector !== "all")
      list = list.filter((a) => a.location?.connector === connector);
    return filterByQuery(list, dq, (a) => [a.name, a.fqn, a.id, a.location?.uri]);
  }, [assets.data, kind, connector, dq]);

  if (assets.isLoading) return <Loading />;

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Datasets</h1>
          <p className="mt-1 text-sm text-slate-500">
            Catalog assets from discovery and registration.
          </p>
        </div>
        <button
          type="button"
          className="btn-primary"
          disabled={seed.isPending}
          onClick={() => seed.mutate()}
        >
          <Sparkles className="h-4 w-4" /> Load mock datasets
        </button>
      </div>

      <div className="flex flex-col gap-3 lg:flex-row">
        <SearchInput
          value={q}
          onChange={setQ}
          placeholder="Search name, FQN, id…"
        />
        <select
          className="input lg:max-w-[160px]"
          value={kind}
          onChange={(e) => setKind(e.target.value)}
        >
          {kinds.map((k) => (
            <option key={k} value={k}>
              {k === "all" ? "All kinds" : k}
            </option>
          ))}
        </select>
        <select
          className="input lg:max-w-[160px]"
          value={connector}
          onChange={(e) => setConnector(e.target.value)}
        >
          {connectors.map((c) => (
            <option key={c} value={c}>
              {c === "all" ? "All connectors" : c}
            </option>
          ))}
        </select>
      </div>

      {!items.length ? (
        <EmptyState
          title="No datasets found"
          description="Load mock data or discover a connector source."
          action={
            <button type="button" className="btn-primary" onClick={() => seed.mutate()}>
              Load mock data
            </button>
          }
        />
      ) : (
        <div className="card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead className="bg-slate-50 text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">FQN</th>
                  <th className="px-4 py-3 font-medium">Kind</th>
                  <th className="px-4 py-3 font-medium">Connector</th>
                  <th className="px-4 py-3 font-medium">Updated</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {items.map((a) => (
                  <tr key={a.id} className="hover:bg-slate-50/80">
                    <td className="px-4 py-3">
                      <Link
                        to={`/datasets/${a.id}`}
                        className="font-medium text-brand-700 hover:underline"
                      >
                        {a.name}
                      </Link>
                      <div className="font-mono text-[11px] text-slate-400">
                        {shortId(a.id, 12)}
                      </div>
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-slate-600">
                      {a.fqn}
                    </td>
                    <td className="px-4 py-3 capitalize">{a.kind}</td>
                    <td className="px-4 py-3">{a.location?.connector}</td>
                    <td className="px-4 py-3 text-slate-500">
                      {formatWhen(a.updated_at)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="border-t border-slate-100 px-4 py-2 text-xs text-slate-500">
            {items.length} of {assets.data?.count ?? 0} datasets
          </div>
        </div>
      )}
    </div>
  );
}
