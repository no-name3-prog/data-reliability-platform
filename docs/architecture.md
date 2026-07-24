# Architecture

## Crate dependency graph

```text
drp-common
    ▲
drp-core          (domain · plugins · events · Platform)
    ▲
    ├─ drp-storage
    ├─ drp-connectors
    ├─ drp-notifications
    ├─ drp-metadata
    ├─ drp-profiling
    ├─ drp-validation
    ├─ drp-lineage
    └─ drp-scheduler
         ▲
      drp-api         (composition root · HTTP · binary)
```

## Plugin extension points

| Trait | Built-ins |
|-------|-----------|
| `ConnectorPlugin` | `mock` |
| `ProfilerPlugin` | `basic` |
| `ValidatorPlugin` | `not_null`, `unique`, `regex` |
| `NotificationPlugin` | `log` |
| `JobHandler` | `noop` |

Register new plugins in `drp_api::app::build_app` without changing feature services.

## Runtime topology (compose)

```text
[client] → api:8080 → memory store (default)
                   ↘ config.infra.database_url → postgres:5432
                   ↘ config.infra.redis_url    → redis:6379
                   ↘ config.infra.s3_endpoint  → minio:9000
```

Postgres/Redis/MinIO are **available from day one** for future backends; the default app store is in-memory so unit tests stay fast inside the `dev` container.
