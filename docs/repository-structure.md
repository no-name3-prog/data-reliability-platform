# Repository structure

```text
data-reliability-platform/
├── Cargo.toml                 # Workspace root + shared deps
├── Makefile                   # Container-first DX (only Git + Docker on host)
├── docker-compose.yml         # postgres, redis, minio, api, prometheus, dev
├── docker/
│   ├── Dockerfile             # Production API image
│   ├── Dockerfile.dev         # Toolchain: rustc, clippy, rustfmt, nextest
│   ├── Dockerfile.docs
│   ├── init-postgres.sql
│   └── prometheus/
├── config/                    # Runtime defaults (mounted in containers)
├── .config/nextest.toml       # Test profiles (unit / integration / regression / ci)
├── .githooks/                 # pre-commit / pre-push → make (Docker)
├── .github/workflows/ci.yml   # Mirrors make verify + image smoke
├── docs/
│   ├── plugin-architecture.md # Trait design & rules (start here for plugins)
│   ├── contributing-plugins.md
│   ├── repository-structure.md
│   ├── architecture.md
│   ├── development.md
│   ├── testing.md
│   ├── container-workflow.md
│   └── operations.md
├── scripts/
│   ├── cargo.sh               # cargo inside dev container
│   ├── drp.sh                 # unified helper
│   └── install-hooks.sh
├── crates/                    # Platform libraries & services
│   ├── drp-common/            # Errors, IDs, config, shared value types
│   ├── drp-core/              # Domain + plugin traits + registry + events
│   ├── drp-storage/           # Store trait + memory backend
│   ├── drp-connectors/        # Built-in connectors (mock, fixture, …)
│   ├── drp-metadata/          # Catalog service (uses ConnectorPlugin by id)
│   ├── drp-profiling/         # Profiler plugins + service
│   ├── drp-validation/        # Validator plugins + service
│   ├── drp-anomaly/           # Anomaly detector plugins + service
│   ├── drp-ai/                # AI provider plugins + service
│   ├── drp-notifications/     # Notification plugins + service
│   ├── drp-lineage/           # Lineage graph
│   ├── drp-scheduler/         # Jobs + JobHandler registry
│   ├── drp-api/               # Composition root + HTTP + binary
│   ├── drp-test-support/      # Shared test harness (not published)
│   └── drp-tests/             # Integration + regression suites
└── plugins/                   # Optional / example / third-party-style plugins
    └── example-connector/     # Copy-paste template
```

## Where new code goes

| You are adding… | Put it in… | Touch core? |
|-----------------|------------|-------------|
| Connector / source | `plugins/<name>` or `crates/drp-connector-*` | No (only `drp-api` register) |
| Validation rule | `plugins/<name>` or `drp-validation` (universal only) | No |
| Anomaly detector | `plugins/<name>` or `drp-anomaly` | No |
| Notification channel | `plugins/<name>` or `drp-notifications` | No |
| AI provider | `plugins/<name>` or `drp-ai` | No |
| New plugin *category* | `drp-core` traits + registry | **Yes** (design review) |
| HTTP route | `drp-api` | No (unless new domain) |
| Cross-cutting type | `drp-common` or `drp-core::domain` | Careful / review |

## Composition root

`crates/drp-api/src/app.rs` owns `register_all_plugins`. This is the **single chokepoint** that knows concrete plugin crates. Feature services remain plugin-agnostic.
