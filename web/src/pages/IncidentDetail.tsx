import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { useState } from "react";
import { api } from "@/api/client";
import { Loading, EmptyState } from "@/components/Loading";
import { StatusBadge, SeverityBadge } from "@/components/StatusBadge";
import { formatWhen, shortId } from "@/lib/utils";

export function IncidentDetailPage() {
  const { id = "" } = useParams();
  const [owner, setOwner] = useState("");
  const [note, setNote] = useState("");
  const qc = useQueryClient();

  const incident = useQuery({
    queryKey: ["incident", id],
    queryFn: () => api.getIncident(id),
    enabled: !!id,
  });
  const history = useQuery({
    queryKey: ["incident-history", id],
    queryFn: () => api.incidentHistory(id),
    enabled: !!id,
  });

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ["incident", id] });
    qc.invalidateQueries({ queryKey: ["incident-history", id] });
    qc.invalidateQueries({ queryKey: ["incidents"] });
  };

  const setStatus = useMutation({
    mutationFn: (status: string) => api.setIncidentStatus(id, status, note || undefined),
    onSuccess: invalidate,
  });
  const assign = useMutation({
    mutationFn: () => api.assignOwner(id, owner),
    onSuccess: () => {
      setOwner("");
      invalidate();
    },
  });

  if (incident.isLoading) return <Loading />;
  if (incident.isError || !incident.data)
    return (
      <EmptyState
        title="Incident not found"
        action={
          <Link to="/incidents" className="btn-secondary">
            Back
          </Link>
        }
      />
    );

  const i = incident.data;
  const events = history.data?.items?.length
    ? history.data.items
    : i.timeline || [];

  return (
    <div className="space-y-5">
      <Link
        to="/incidents"
        className="inline-flex items-center gap-1 text-sm text-slate-500 hover:text-slate-800"
      >
        <ArrowLeft className="h-4 w-4" /> Incidents
      </Link>

      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">{i.title}</h1>
          <p className="mt-2 max-w-2xl text-sm text-slate-600">{i.message}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            <SeverityBadge severity={i.severity} />
            <StatusBadge status={i.status} />
            <span className="badge bg-slate-100 text-slate-600">
              {(i.source as { type?: string })?.type || "unknown"}
            </span>
          </div>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="card space-y-3 p-4 lg:col-span-1">
          <h2 className="text-sm font-semibold">Details</h2>
          <dl className="space-y-2 text-sm">
            <Row k="ID" v={shortId(i.id, 16)} mono />
            <Row k="Primary asset" v={shortId(i.asset_id, 14)} mono />
            <Row k="Owner" v={i.owner || "Unassigned"} />
            <Row k="Detector" v={i.detector || "—"} />
            <Row k="Created" v={formatWhen(i.created_at)} />
            <Row k="Updated" v={formatWhen(i.updated_at)} />
            <Row
              k="Affected"
              v={`${i.affected_assets?.length || 1} asset(s)`}
            />
          </dl>

          <div className="border-t border-slate-100 pt-3">
            <label className="text-xs font-medium text-slate-500">
              Assign owner
            </label>
            <div className="mt-1 flex gap-2">
              <input
                className="input"
                value={owner}
                onChange={(e) => setOwner(e.target.value)}
                placeholder="owner@example.com"
              />
              <button
                type="button"
                className="btn-secondary shrink-0"
                disabled={!owner || assign.isPending}
                onClick={() => assign.mutate()}
              >
                Save
              </button>
            </div>
          </div>

          <div>
            <label className="text-xs font-medium text-slate-500">
              Update status
            </label>
            <div className="mt-2 flex flex-wrap gap-2">
              {["open", "in_progress", "acknowledged", "monitoring", "resolved"].map(
                (s) => (
                  <button
                    key={s}
                    type="button"
                    className="btn-secondary capitalize"
                    disabled={setStatus.isPending}
                    onClick={() => setStatus.mutate(s)}
                  >
                    {s.replaceAll("_", " ")}
                  </button>
                ),
              )}
            </div>
            <textarea
              className="input mt-2 min-h-[70px]"
              placeholder="Optional note for status change"
              value={note}
              onChange={(e) => setNote(e.target.value)}
            />
          </div>
        </div>

        <div className="card p-4 lg:col-span-2">
          <h2 className="text-sm font-semibold">Timeline history</h2>
          <ol className="relative mt-4 space-y-4 border-l border-slate-200 pl-5">
            {[...events].reverse().map((e) => (
              <li key={e.id} className="relative">
                <span className="absolute -left-[1.4rem] top-1 h-2.5 w-2.5 rounded-full bg-brand-500 ring-4 ring-white" />
                <div className="rounded-xl bg-slate-50 px-3 py-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <span className="text-xs font-semibold uppercase tracking-wide text-brand-700">
                      {e.event_type.replaceAll("_", " ")}
                    </span>
                    <span className="text-xs text-slate-400">
                      {formatWhen(e.at)}
                    </span>
                  </div>
                  <p className="mt-1 text-sm text-slate-700">{e.message}</p>
                  {e.actor ? (
                    <p className="mt-0.5 text-xs text-slate-400">by {e.actor}</p>
                  ) : null}
                </div>
              </li>
            ))}
            {!events.length ? (
              <li className="text-sm text-slate-500">No timeline events</li>
            ) : null}
          </ol>
        </div>
      </div>
    </div>
  );
}

function Row({
  k,
  v,
  mono,
}: {
  k: string;
  v: string;
  mono?: boolean;
}) {
  return (
    <div>
      <dt className="text-xs text-slate-500">{k}</dt>
      <dd className={mono ? "font-mono text-xs" : ""}>{v}</dd>
    </div>
  );
}
