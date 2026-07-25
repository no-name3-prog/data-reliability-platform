# Connectors

## In plain English

A **connector** is how the platform talks to a data system
(mock sample data, Postgres, CSV files, Parquet files, …).

**Discover** finds tables/files. Then you can profile and validate them.

Built-in for demos: connector id **`mock`** (orders + users sample data).

---

## Interface

All connectors implement `drp_core::ConnectorPlugin`:

| Method | Purpose |
|--------|---------|
| `test_connection` | Validate credentials / path |
| `discover` | Flat list of assets |
| `discover_catalog` | Hierarchical DB → schema → table → columns |
| `sample_rows` | Fetch sample JSON rows for profiling / checks |

New databases: implement the trait in a new crate and call `register` from `drp-api` composition root.

## Built-in connectors

### `postgres`

- **URI**: `postgres://user:pass@host:5432/dbname`
- Discovers non-system schemas, tables/views, columns via `information_schema`
- Samples with `SELECT * FROM "schema"."table" LIMIT n`
- Optional property `schemas=public,analytics` to filter

### `csv`

- **URI**: path to a `.csv` file or directory of CSV files
- Infers columns from header + sample values
- FQN: `files.csv.<stem>`

### `parquet`

- **URI**: path to a `.parquet` file or directory
- Uses Arrow schema for column types
- FQN: `files.parquet.<stem>`

### Test connectors

`mock`, `fixture`, `failing` — see testing docs.

## Metadata storage

Discovered assets (with columns) are persisted through `Store`:

- `memory` — default for unit tests
- `postgres` — set `DRP_STORAGE_BACKEND=postgres` and `DRP_DATABASE_URL` (compose provides Postgres)

```bash
# inside compose network
DRP_STORAGE_BACKEND=postgres
DRP_DATABASE_URL=postgres://drp:drp@postgres:5432/drp
```

## API

```http
POST /v1/assets/discover
POST /v1/assets/catalog
{
  "connector": "csv",
  "uri": "/workspace/crates/drp-connectors/testdata",
  "properties": {}
}
```

## Adding a database (checklist)

1. New crate under `plugins/` or `crates/drp-connector-*`
2. Implement `ConnectorPlugin` (+ rich `discover_catalog`)
3. Unit tests with a disposable source
4. `register` in composition root
5. `make test` / `make lint` (Docker)
