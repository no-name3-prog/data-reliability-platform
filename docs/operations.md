# Operations

## Health checks

Use readiness for load balancers and liveness for orchestrators:

```bash
curl -fsS http://127.0.0.1:8080/livez
curl -fsS http://127.0.0.1:8080/readyz
```

Compose `api` healthcheck uses `/readyz`.

## Metrics scrape

Prometheus (compose) scrapes:

```yaml
targets: ["api:8080"]
metrics_path: /metrics
```

Config: `docker/prometheus/prometheus.yml`.

## Logging

Prefer `DRP_LOG_FORMAT=json` in shared environments. Fields include span data for HTTP (`method`, `uri`, `request_id`).

## Configuration

Layering (later wins):

1. `config/default.toml`
2. `config/{DRP_ENV}.toml` (optional)
3. `DRP_*` environment variables

See `.env.example` for compose-oriented variables.

## Backups / volumes

Named Docker volumes hold infra state (`drp-postgres`, `drp-redis`, `drp-minio`, `drp-prometheus`).  
`make clean` **destroys** volumes — local data only.
