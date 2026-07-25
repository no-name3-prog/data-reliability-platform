import { NavLink, Outlet } from "react-router-dom";
import {
  Activity,
  AlertTriangle,
  Database,
  GitBranch,
  LayoutDashboard,
  Menu,
  Plug,
  ShieldCheck,
  X,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { cn } from "@/lib/utils";

const nav = [
  { to: "/", label: "Overview", icon: LayoutDashboard, end: true },
  { to: "/sources", label: "Data sources", icon: Plug },
  { to: "/datasets", label: "Datasets", icon: Database },
  { to: "/profiling", label: "Profiling", icon: Activity },
  { to: "/validation", label: "Validation", icon: ShieldCheck },
  { to: "/lineage", label: "Lineage", icon: GitBranch },
  { to: "/incidents", label: "Incidents", icon: AlertTriangle },
];

export function Layout() {
  const [open, setOpen] = useState(false);
  const health = useQuery({ queryKey: ["health"], queryFn: api.health, refetchInterval: 15000 });

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-white to-indigo-50/40">
      {/* mobile top bar */}
      <header className="sticky top-0 z-30 flex items-center justify-between border-b border-slate-200/80 bg-white/90 px-4 py-3 backdrop-blur lg:hidden">
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="rounded-lg p-2 hover:bg-slate-100"
            onClick={() => setOpen(true)}
            aria-label="Open menu"
          >
            <Menu className="h-5 w-5" />
          </button>
          <Brand />
        </div>
        <HealthDot ok={health.data} />
      </header>

      <div className="mx-auto flex max-w-[1400px]">
        {/* desktop sidebar */}
        <aside className="sticky top-0 hidden h-screen w-64 shrink-0 flex-col border-r border-slate-200/80 bg-white/70 px-3 py-5 backdrop-blur lg:flex">
          <div className="mb-6 flex items-center justify-between px-2">
            <Brand />
            <HealthDot ok={health.data} />
          </div>
          <Nav onNavigate={() => undefined} />
          <p className="mt-auto px-3 pt-6 text-[11px] leading-relaxed text-slate-400">
            Data Reliability Platform
            <br />
            Container-first · API-driven
          </p>
        </aside>

        {/* mobile drawer */}
        {open ? (
          <div className="fixed inset-0 z-40 lg:hidden">
            <button
              type="button"
              className="absolute inset-0 bg-slate-900/40"
              aria-label="Close menu overlay"
              onClick={() => setOpen(false)}
            />
            <aside className="absolute left-0 top-0 flex h-full w-72 flex-col bg-white p-4 shadow-xl">
              <div className="mb-4 flex items-center justify-between">
                <Brand />
                <button
                  type="button"
                  className="rounded-lg p-2 hover:bg-slate-100"
                  onClick={() => setOpen(false)}
                >
                  <X className="h-5 w-5" />
                </button>
              </div>
              <Nav onNavigate={() => setOpen(false)} />
            </aside>
          </div>
        ) : null}

        <main className="min-w-0 flex-1 px-4 py-5 sm:px-6 lg:px-8 lg:py-8">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

function Brand() {
  return (
    <div className="flex items-center gap-2.5">
      <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-brand-500 to-brand-700 text-sm font-bold text-white shadow-sm">
        DR
      </div>
      <div>
        <div className="text-sm font-semibold tracking-tight text-slate-900">
          Reliability
        </div>
        <div className="text-[11px] text-slate-500">Platform console</div>
      </div>
    </div>
  );
}

function HealthDot({ ok }: { ok?: boolean }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-[11px] font-medium",
        ok ? "bg-emerald-50 text-emerald-700" : "bg-slate-100 text-slate-500",
      )}
      title={ok ? "API ready" : "API unreachable"}
    >
      <span
        className={cn(
          "h-1.5 w-1.5 rounded-full",
          ok ? "bg-emerald-500" : "bg-slate-400",
        )}
      />
      {ok ? "Live" : "Offline"}
    </span>
  );
}

function Nav({ onNavigate }: { onNavigate: () => void }) {
  return (
    <nav className="flex flex-col gap-0.5">
      {nav.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.end}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              "flex items-center gap-2.5 rounded-xl px-3 py-2.5 text-sm font-medium transition",
              isActive
                ? "bg-brand-50 text-brand-700"
                : "text-slate-600 hover:bg-slate-50 hover:text-slate-900",
            )
          }
        >
          <item.icon className="h-4 w-4 shrink-0 opacity-80" />
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}
