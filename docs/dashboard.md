# Platform dashboard

Modern web console for browsing sources, datasets, profiling, validation, lineage, and incidents.

## Run (container-first)

```bash
make bootstrap   # first time
make up          # API + infra + dashboard
```

Open **http://127.0.0.1:3000**

| Service | URL |
|---------|-----|
| Dashboard | http://127.0.0.1:3000 |
| API | http://127.0.0.1:8080 |
| Prometheus | http://127.0.0.1:9090 |

Only dashboard (rebuild static UI):

```bash
make web-build
make web
```

## Features

- **Overview** — KPIs, charts (dataset kinds, incident severity, validation suites), load mock data
- **Data sources** — connector plugins with search + capability filters
- **Datasets** — catalog table with search / kind / connector filters + detail page
- **Profiling** — column stats, null/distinct charts, history, re-run profile
- **Validation** — checks, suite runs, demo not-null check on mock orders
- **Lineage** — SVG dependency graph, SQL ingest, impact panel
- **Incidents** — search/filter, detail with timeline, owner, status updates

## Local frontend dev (optional)

If you have Node 20+ on the host (optional; Docker is preferred):

```bash
make up          # API on :8080
cd web && npm install && npm run dev
# http://127.0.0.1:3000 with Vite proxy → API
```

## Architecture

```text
Browser → nginx (web:3000) → static SPA
                 └─ /v1/* proxy → api:8080
```

Stack: React 18, Vite, TypeScript, Tailwind, TanStack Query, Recharts.
