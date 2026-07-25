import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "@/api/client";
import { SearchInput } from "@/components/SearchInput";
import { EmptyState, Loading } from "@/components/Loading";
import { StatusBadge, SeverityBadge } from "@/components/StatusBadge";
import { filterByQuery, formatWhen } from "@/lib/utils";
import { useDebounced } from "@/hooks/useDebounced";

export function IncidentsPage() {
  const [q, setQ] = useState("");
  const [status, setStatus] = useState("all");
  const [severity, setSeverity] = useState("all");
  const dq = useDebounced(q);

  const incidents = useQuery({
    queryKey: ["incidents"],
    queryFn: () => api.listIncidents(undefined, 200),
  });

  const items = useMemo(() => {
    let list = incidents.data?.items ?? [];
    if (status !== "all")
      list = list.filter((i) => i.status.toLowerCase() === status);
    if (severity !== "all")
      list = list.filter((i) => i.severity.toLowerCase() === severity);
    return filterByQuery(list, dq, (i) => [
      i.title,
      i.message,
      i.owner,
      i.id,
      i.detector,
    ]);
  }, [incidents.data, status, severity, dq]);

  if (incidents.isLoading) return <Loading />;

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Incidents</h1>
        <p className="mt-1 text-sm text-slate-500">
          Failures and anomalies with severity, owners, and timeline history.
        </p>
      </div>

      <div className="flex flex-col gap-3 lg:flex-row">
        <SearchInput
          value={q}
          onChange={setQ}
          placeholder="Search title, owner, detector…"
        />
        <select
          className="input lg:max-w-[160px]"
          value={status}
          onChange={(e) => setStatus(e.target.value)}
        >
          <option value="all">All statuses</option>
          {["open", "in_progress", "acknowledged", "monitoring", "resolved"].map(
            (s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ),
          )}
        </select>
        <select
          className="input lg:max-w-[160px]"
          value={severity}
          onChange={(e) => setSeverity(e.target.value)}
        >
          <option value="all">All severities</option>
          {["critical", "high", "medium", "low"].map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      {!items.length ? (
        <EmptyState
          title="No incidents match"
          description="Run a failing validation check on mock data to open incidents."
        />
      ) : (
        <div className="card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead className="bg-slate-50 text-xs uppercase text-slate-500">
                <tr>
                  <th className="px-4 py-3">Title</th>
                  <th className="px-4 py-3">Severity</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Owner</th>
                  <th className="px-4 py-3">Updated</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {items.map((i) => (
                  <tr key={i.id} className="hover:bg-slate-50/80">
                    <td className="px-4 py-3">
                      <Link
                        to={`/incidents/${i.id}`}
                        className="font-medium text-brand-700 hover:underline"
                      >
                        {i.title}
                      </Link>
                      <div className="text-xs text-slate-400">
                        {(i.source as { type?: string })?.type || "—"} ·{" "}
                        {i.affected_assets?.length || 1} assets
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <SeverityBadge severity={i.severity} />
                    </td>
                    <td className="px-4 py-3">
                      <StatusBadge status={i.status} />
                    </td>
                    <td className="px-4 py-3 text-slate-600">
                      {i.owner || "—"}
                    </td>
                    <td className="px-4 py-3 text-slate-500">
                      {formatWhen(i.updated_at)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
