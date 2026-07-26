# Architecture

## In plain English

The backend is split into small **Rust packages** (crates).  
Each package has a job (storage, validation, lineage, …).  
They plug together through a shared **API process**.

You can add new connectors or rules without rewriting the whole app (**plugins**).

The **dashboard** is a separate web app that calls the API.

---

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
| `AiProviderPlugin` | `heuristic`, `echo`, optional `openai_compatible` |
| `JobHandler` (scheduler) | `noop` |

New plugins: implement trait → `register` helper → **one line** in `register_all_plugins`.

Optional AI rule suggestions (human-in-the-loop): [ai-rule-suggestions.md](./ai-rule-suggestions.md).

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
