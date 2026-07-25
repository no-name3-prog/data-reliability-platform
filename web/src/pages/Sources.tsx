import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plug, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import { SearchInput } from "@/components/SearchInput";
import { EmptyState, Loading } from "@/components/Loading";
import { filterByQuery } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";

export function SourcesPage() {
  const [q, setQ] = useState("");
  const dq = useDebounced(q);
  const [cap, setCap] = useState<string>("all");
  const qc = useQueryClient();
  const plugins = useQuery({ queryKey: ["plugins"], queryFn: api.listPlugins });
  const seed = useMutation({
    mutationFn: api.discoverMock,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["assets"] }),
  });

  const items = useMemo(() => {
    let list = plugins.data?.items ?? [];
    if (cap !== "all") {
      list = list.filter((p) => p.capabilities?.includes(cap));
    }
    return filterByQuery(list, dq, (p) => [p.id, p.name, p.description]);
  }, [plugins.data, dq, cap]);

  const caps = useMemo(() => {
    const s = new Set<string>();
    (plugins.data?.items ?? []).forEach((p) =>
      p.capabilities?.forEach((c) => s.add(c)),
    );
    return ["all", ...Array.from(s).sort()];
  }, [plugins.data]);

  if (plugins.isLoading) return <Loading />;

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Data sources</h1>
          <p className="mt-1 text-sm text-slate-500">
            Registered connector plugins and capabilities.
          </p>
        </div>
        <button
          type="button"
          className="btn-primary"
          onClick={() => seed.mutate()}
          disabled={seed.isPending}
        >
          <Sparkles className="h-4 w-4" /> Discover mock source
        </button>
      </div>

      <div className="flex flex-col gap-3 sm:flex-row">
        <SearchInput
          value={q}
          onChange={setQ}
          placeholder="Search plugins by name or id…"
        />
        <select
          className="input sm:max-w-[200px]"
          value={cap}
          onChange={(e) => setCap(e.target.value)}
        >
          {caps.map((c) => (
            <option key={c} value={c}>
              {c === "all" ? "All capabilities" : c}
            </option>
          ))}
        </select>
      </div>

      {!items.length ? (
        <EmptyState
          title="No plugins match"
          description="Adjust filters or ensure the API is running."
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {items.map((p) => (
            <article key={p.id} className="card p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-xl bg-brand-50 p-2.5 text-brand-600">
                  <Plug className="h-5 w-5" />
                </div>
                <div className="min-w-0">
                  <h2 className="font-semibold text-slate-900">{p.name}</h2>
                  <p className="font-mono text-xs text-slate-500">{p.id}</p>
                  <p className="mt-2 text-sm text-slate-600">
                    {p.description || "No description"}
                  </p>
                  <div className="mt-3 flex flex-wrap gap-1.5">
                    {(p.capabilities || []).map((c) => (
                      <span
                        key={c}
                        className="badge bg-slate-100 text-slate-600"
                      >
                        {c}
                      </span>
                    ))}
                    <span className="badge bg-slate-50 text-slate-400">
                      v{p.version}
                    </span>
                  </div>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
