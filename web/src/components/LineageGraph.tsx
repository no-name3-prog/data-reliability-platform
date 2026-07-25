import { useMemo } from "react";
import type { LineageEdge, LineageNode } from "@/api/types";

/** Lightweight SVG force-free layered graph for lineage snapshots. */
export function LineageGraph({
  nodes,
  edges,
  highlightId,
  onSelect,
}: {
  nodes: LineageNode[];
  edges: LineageEdge[];
  highlightId?: string;
  onSelect?: (id: string) => void;
}) {
  const layout = useMemo(() => layoutGraph(nodes, edges), [nodes, edges]);

  if (!nodes.length) {
    return (
      <div className="flex h-64 items-center justify-center text-sm text-slate-500">
        No lineage nodes yet. Parse SQL or register assets.
      </div>
    );
  }

  const { positions, width, height } = layout;

  return (
    <div className="w-full overflow-auto rounded-xl border border-slate-100 bg-slate-50/50">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="min-h-[320px] w-full min-w-[640px]"
        role="img"
        aria-label="Lineage graph"
      >
        <defs>
          <marker
            id="arrow"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8" />
          </marker>
        </defs>
        {edges.map((e, i) => {
          const a = positions.get(e.from);
          const b = positions.get(e.to);
          if (!a || !b) return null;
          return (
            <g key={`${e.from}-${e.to}-${i}`}>
              <line
                x1={a.x}
                y1={a.y}
                x2={b.x}
                y2={b.y}
                stroke="#94a3b8"
                strokeWidth={1.5}
                markerEnd="url(#arrow)"
              />
              <text
                x={(a.x + b.x) / 2}
                y={(a.y + b.y) / 2 - 6}
                textAnchor="middle"
                className="fill-slate-400"
                style={{ fontSize: 10 }}
              >
                {e.kind}
              </text>
            </g>
          );
        })}
        {nodes.map((n) => {
          const p = positions.get(n.asset_id);
          if (!p) return null;
          const active = highlightId === n.asset_id;
          return (
            <g
              key={n.asset_id}
              transform={`translate(${p.x}, ${p.y})`}
              className="cursor-pointer"
              onClick={() => onSelect?.(n.asset_id)}
            >
              <rect
                x={-70}
                y={-22}
                width={140}
                height={44}
                rx={12}
                fill={active ? "#eef2ff" : "white"}
                stroke={active ? "#6366f1" : "#e2e8f0"}
                strokeWidth={active ? 2 : 1}
              />
              <text
                textAnchor="middle"
                y={-2}
                style={{ fontSize: 11, fontWeight: 600 }}
                className="fill-slate-800"
              >
                {truncate(n.label || n.fqn || n.asset_id, 18)}
              </text>
              <text
                textAnchor="middle"
                y={14}
                style={{ fontSize: 10 }}
                className="fill-slate-400"
              >
                {n.kind || n.node_type || "node"}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function truncate(s: string, n: number) {
  return s.length > n ? `${s.slice(0, n - 1)}…` : s;
}

function layoutGraph(nodes: LineageNode[], edges: LineageEdge[]) {
  const ids = nodes.map((n) => n.asset_id);
  const indeg = new Map(ids.map((id) => [id, 0]));
  const outs = new Map<string, string[]>(ids.map((id) => [id, []]));
  for (const e of edges) {
    if (!indeg.has(e.from) || !indeg.has(e.to)) continue;
    indeg.set(e.to, (indeg.get(e.to) || 0) + 1);
    outs.get(e.from)!.push(e.to);
  }

  // Kahn layering
  const layer = new Map<string, number>();
  let frontier = ids.filter((id) => (indeg.get(id) || 0) === 0);
  frontier.forEach((id) => layer.set(id, 0));
  const q = [...frontier];
  while (q.length) {
    const u = q.shift()!;
    const lu = layer.get(u) || 0;
    for (const v of outs.get(u) || []) {
      layer.set(v, Math.max(layer.get(v) || 0, lu + 1));
      indeg.set(v, (indeg.get(v) || 1) - 1);
      if (indeg.get(v) === 0) q.push(v);
    }
  }
  ids.forEach((id) => {
    if (!layer.has(id)) layer.set(id, 0);
  });

  const byLayer = new Map<number, string[]>();
  for (const id of ids) {
    const L = layer.get(id) || 0;
    if (!byLayer.has(L)) byLayer.set(L, []);
    byLayer.get(L)!.push(id);
  }

  const colW = 200;
  const rowH = 90;
  const pad = 60;
  const maxLayer = Math.max(...[...byLayer.keys()], 0);
  const maxRows = Math.max(...[...byLayer.values()].map((v) => v.length), 1);
  const width = pad * 2 + (maxLayer + 1) * colW;
  const height = pad * 2 + maxRows * rowH;

  const positions = new Map<string, { x: number; y: number }>();
  for (const [L, list] of byLayer) {
    list.forEach((id, i) => {
      positions.set(id, {
        x: pad + L * colW + colW / 2,
        y: pad + i * rowH + rowH / 2,
      });
    });
  }

  return { positions, width, height };
}
