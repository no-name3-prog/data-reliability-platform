# Lineage engine

## Overview

The lineage engine builds a **dependency graph** from:

1. **SQL parsing** — `CREATE VIEW` / `CREATE TABLE AS` / `INSERT … SELECT` / plain `SELECT`
2. **Manual edges** — API-declared transforms
3. **Dashboards & pipelines** — first-class node kinds linked to tables/datasets

It supports **table-level** and **column-level** lineage, **upstream/downstream** traversal, and **impact analysis** when a table changes or a validation fails.

## Architecture

```text
SQL  ──► sql_parse::extract_lineage_from_sql
              │
              ▼
       LineageService::ingest_sql
              │
     ┌────────┴────────┐
     ▼                 ▼
 table edges      column edges
     │                 │
     └────────┬────────┘
              ▼
         LineageGraph (petgraph)
              │
              ├── upstream / downstream
              └── impact (datasets, dashboards, pipelines)
```

## Node kinds

`table`, `view`, `dataset`, `dashboard`, `pipeline`, `file`, `other`

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/lineage` | Full snapshot |
| POST | `/v1/lineage/edges` | Add table-level edge |
| POST | `/v1/lineage/nodes` | Register node (dashboard/pipeline/…) |
| POST | `/v1/lineage/parse-sql` | Parse SQL → edges |
| GET | `/v1/lineage/assets/{id}/upstream` | Upstream deps |
| GET | `/v1/lineage/assets/{id}/downstream` | Downstream deps |
| GET | `/v1/lineage/assets/{id}/columns/{col}/upstream` | Column upstream |
| GET | `/v1/lineage/assets/{id}/columns/{col}/downstream` | Column downstream |
| GET | `/v1/lineage/assets/{id}/impact` | Impact of table change |
| POST | `/v1/lineage/impact/validation` | Impact of validation failure |

### Parse SQL example

```json
POST /v1/lineage/parse-sql
{
  "sql": "CREATE VIEW analytics.orders_enriched AS SELECT o.id, u.email FROM raw.orders o JOIN raw.users u ON o.user_id = u.id"
}
```

### Validation impact example

```json
POST /v1/lineage/impact/validation
{
  "asset_id": "<ulid>",
  "check_id": "<check>",
  "message": "not_null failed on email"
}
```

Returns datasets, dashboards, and pipelines downstream of the asset.

## Configuration

```toml
[lineage]
max_depth = 20
```
