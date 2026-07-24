# Development guide

## Container-first rule

| Allowed on host | Forbidden on host |
|-----------------|-------------------|
| Git | `cargo`, `rustc`, `rustup` |
| Docker / Compose | Local Postgres / Redis / MinIO |
| Editor / IDE | Host-side test runs for this repo |
| `make` / `./scripts/*` | Ad-hoc host package installs for the stack |

## Service map (`make up`)

| Service | Port | Purpose |
|---------|------|---------|
| `api` | 8080 | DRP HTTP API |
| `postgres` | 5432 | Future SQL backends |
| `redis` | 6379 | Future cache / queues |
| `minio` | 9000 / 9001 | S3-compatible object storage |
| `prometheus` | 9090 | Scrapes `api:8080/metrics` |

## Observability

### Structured logging

- Config: `config/default.toml` → `[logging]`
- Env: `DRP_LOG_LEVEL`, `DRP_LOG_FORMAT` (`pretty` \| `json`)
- API containers default to **JSON** logs for aggregation
- Requests carry `x-request-id` (generated + propagated)

### Health probes

| Path | Semantics |
|------|-----------|
| `/livez`, `/health`, `/startupz` | Process up |
| `/readyz`, `/ready` | Plugins registered + store reachable |

### Metrics

- `GET /metrics` — Prometheus text format
- Series include `http_requests_total`, `http_request_duration_seconds`, `http_responses_total`
- Prometheus UI: http://127.0.0.1:9090

## Tooling

| File | Role |
|------|------|
| `rustfmt.toml` | Formatting |
| `clippy.toml` | Lint thresholds |
| `.editorconfig` | Cross-editor basics |
| `deny.toml` | License / advisory policy |
| `.githooks/` | Container-backed git hooks |
| `.pre-commit-config.yaml` | Optional pre-commit framework |

## Documentation generation

```bash
make doc          # cargo doc inside container
make docs-serve   # nginx on :3001
```
