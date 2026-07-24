# Architecture

## Overview

The platform is a **Cargo workspace** of small crates with a **trait-based plugin system**. Core defines contracts; implementations register at the **composition root** (`drp-api`).

For plugins specifically, read **[plugin-architecture.md](./plugin-architecture.md)**.

## Crate dependency graph

```text
drp-common
    ▲
drp-core            domain · plugin traits · PluginRegistry · events · Platform
    ▲
    ├─ drp-storage
    ├─ drp-connectors      ─┐
    ├─ drp-profiling         │ plugin implementation crates
    ├─ drp-validation        │ (also: plugins/*)
    ├─ drp-anomaly           │
    ├─ drp-ai                │
    ├─ drp-notifications   ─┘
    ├─ drp-metadata        (services: use registry by id)
    ├─ drp-lineage
    └─ drp-scheduler
         ▲
      drp-api              composition root + HTTP + binary
```

## Plugin extension points

| Trait | Built-ins (ids) |
|-------|-----------------|
| `ConnectorPlugin` | `mock`, `fixture`, `example` |
| `ProfilerPlugin` | `basic` |
| `ValidatorPlugin` | `not_null`, `unique`, `regex` |
| `AnomalyDetectorPlugin` | `null_spike`, `zscore` |
| `NotificationPlugin` | `log` |
| `AiProviderPlugin` | `echo` |
| `JobHandler` (scheduler) | `noop` |

New plugins: implement trait → `register` helper → **one line** in `register_all_plugins`.

## Runtime topology (compose)

```text
[client] → api:8080 → memory store (default)
                   ↘ postgres / redis / minio (compose DNS)
                   ↘ /metrics ← prometheus:9090
```

## Related docs

- [Repository structure](./repository-structure.md)
- [Contributing plugins](./contributing-plugins.md)
- [Development process](./development-process.md)
- [Testing](./testing.md)
- [Container workflow](./container-workflow.md)
