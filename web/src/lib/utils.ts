import { clsx, type ClassValue } from "clsx";

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs);
}

export function shortId(id?: string | null, n = 8) {
  if (!id) return "—";
  return id.length > n ? `${id.slice(0, n)}…` : id;
}

export function formatWhen(iso?: string | null) {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  } catch {
    return iso;
  }
}

export function statusColor(status: string) {
  const s = status.toLowerCase();
  if (["passed", "healthy", "resolved", "succeeded"].includes(s))
    return "bg-emerald-50 text-emerald-700 ring-emerald-600/15";
  if (["failed", "error", "critical", "unhealthy"].includes(s))
    return "bg-rose-50 text-rose-700 ring-rose-600/15";
  if (["warned", "warning", "open", "degraded", "medium"].includes(s))
    return "bg-amber-50 text-amber-800 ring-amber-600/15";
  if (["in_progress", "acknowledged", "monitoring", "running"].includes(s))
    return "bg-sky-50 text-sky-700 ring-sky-600/15";
  return "bg-slate-50 text-slate-600 ring-slate-500/10";
}

export function severityColor(sev: string) {
  const s = sev.toLowerCase();
  if (s === "critical") return "bg-rose-100 text-rose-800";
  if (s === "high" || s === "error") return "bg-orange-100 text-orange-800";
  if (s === "medium" || s === "warning") return "bg-amber-100 text-amber-800";
  return "bg-slate-100 text-slate-700";
}

export function filterByQuery<T>(
  items: T[],
  q: string,
  fields: (item: T) => Array<string | null | undefined>,
): T[] {
  const needle = q.trim().toLowerCase();
  if (!needle) return items;
  return items.filter((item) =>
    fields(item).some((f) => (f || "").toLowerCase().includes(needle)),
  );
}
