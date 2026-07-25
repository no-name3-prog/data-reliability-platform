# Dashboard (web UI)

The dashboard is the **website** for this platform. You click through pages instead of using only the API.

## How to open it

```bash
make up
```

Then open: **http://127.0.0.1:3000**

The API must also be running (same `make up`). The UI calls the API for all data.

| Service | URL |
|---------|-----|
| Dashboard | http://127.0.0.1:3000 |
| API | http://127.0.0.1:8080 |

## Pages

| Page | What you do there |
|------|-------------------|
| **Overview** | See numbers and charts; **Load mock data** |
| **Data sources** | See connectors (plugins) |
| **Datasets** | Browse tables/files; open one for details |
| **Profiling** | Stats, null rates, history |
| **Validation** | Quality checks and results |
| **Lineage** | Dependency graph; paste SQL; impact |
| **Incidents** | Failures, owners, status, timeline |

## Quick demo

1. Overview → **Load mock data**
2. Datasets → open **orders**
3. Profiling → select **orders** → view charts
4. Validation → **Run demo not-null check** (should fail once)
5. Incidents → open the new item
6. Lineage → **Ingest SQL**

## Rebuild the UI only

```bash
make web-build
make web
```

## Optional: develop the UI with hot reload

Only if you have **Node.js 20+** installed (not required for normal use):

```bash
make up                 # API on 8080
cd web
npm install
npm run dev             # http://127.0.0.1:3000
```

## Tech (for developers)

React, Vite, TypeScript, Tailwind. Code lives in the `web/` folder.  
Docker builds a static site and nginx proxies `/v1` to the API.
