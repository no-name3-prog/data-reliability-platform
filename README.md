# Data Reliability Platform

**Container-first** Rust monorepo for data reliability: catalog metadata, profiling, validation, lineage, scheduling, notifications, structured logging, and Prometheus metrics.

## Host prerequisites (only these)

| Tool | Purpose |
|------|---------|
| **Git** | Version control + hooks |
| **Docker CLI** + **Compose** | Build, lint, test, run, docs, infra |

### Explicitly not required on the host

- Rust toolchain (`rustc`, `cargo`, `rustup`)
- PostgreSQL, Redis, MinIO, Prometheus
- Running tests with host `cargo test`

## Quick start

```bash
git clone https://github.com/no-name3-prog/data-reliability-platform.git
cd data-reliability-platform

make doctor
make bootstrap    # toolchain image + postgres/redis/minio + git hooks
make build
make test
make lint
make up           # api + prometheus + infra
```

| Endpoint | URL |
|----------|-----|
| Readiness | http://127.0.0.1:8080/readyz |
| Liveness | http://127.0.0.1:8080/livez |
| Metrics | http://127.0.0.1:8080/metrics |
| Prometheus | http://127.0.0.1:9090 |
| MinIO console | http://127.0.0.1:9001 |

## Common commands

| Command | Runs in | Description |
|---------|---------|-------------|
| `make bootstrap` | Docker | Image + infra + hooks |
| `make up` / `make down` | Compose | Full stack |
| `make build` | Container | `cargo build --workspace` |
| `make test` | Container | cargo-nextest full suite |
| `make test-unit` | Container | Unit tests (nextest) |
| `make test-integration` | Container | Integration tests |
| `make test-regression` | Container | Regression / golden |
| `make verify` | Container | Full CI mirror |
| `make fmt` / `make lint` | Container | rustfmt + clippy |
| `make check` / `make ci` | Container | Full quality gate |
| `make doc` / `make docs-serve` | Container | rustdoc |
| `make shell` | Container | Interactive toolchain shell |
| `make hooks` | Host git config | Install container-backed hooks |
| `./scripts/cargo.sh …` | Container | Arbitrary cargo |
| `./scripts/drp.sh <target>` | Make/Docker | Unified helper |

## Production-grade standards

| Concern | Implementation |
|---------|----------------|
| Formatting | `rustfmt.toml` + `make fmt` / `fmt-check` |
| Linting | `clippy.toml` + `make clippy` (`-D warnings`) |
| Editor | `.editorconfig` |
| Git hooks | `.githooks/` + optional `.pre-commit-config.yaml` (Docker only) |
| CI | `.github/workflows/ci.yml` (Docker-only runner) |
| Logging | JSON/pretty structured tracing + `x-request-id` |
| Metrics | Prometheus `/metrics` + compose `prometheus` service |
| Health | `/livez`, `/readyz`, `/startupz` |
| Docs | `docs/*`, `CONTRIBUTING.md`, rustdoc |

## Workspace crates

| Crate | Role |
|-------|------|
| `drp-common` | Errors, IDs, config, shared types |
| `drp-core` | Domain, plugins, events, logging |
| `drp-storage` | Persistence trait + memory backend |
| `drp-connectors` | Connector plugins (`mock`) |
| `drp-metadata` | Asset catalog |
| `drp-profiling` | Profiling engine |
| `drp-validation` | Validation engine (rules, schedule, history) |
| `drp-anomaly` | Anomaly detector plugins |
| `drp-ai` | AI / LLM provider plugins |
| `drp-lineage` | Lineage graph |
| `drp-scheduler` | Jobs |
| `drp-notifications` | Alerts |
| `drp-api` | HTTP API + binary |

## Contributing / branching

Direct pushes to `main` are **blocked**. Use a feature branch + PR; **only the repo owner merges**.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/branching-and-merging.md](docs/branching-and-merging.md).

## Plugin system

Connectors, validation rules, anomaly detectors, notifications, and AI providers are **Rust traits** in `drp-core`. Implementations are separate crates registered only at the API composition root.

| Doc | Purpose |
|-----|---------|
| [Validation](docs/validation.md) | Rules, schedule, result history |
| [Plugin architecture](docs/plugin-architecture.md) | Traits, registry, dependency rules |
| [Contributing plugins](docs/contributing-plugins.md) | Step-by-step for new plugins |
| [Repository structure](docs/repository-structure.md) | Where code lives |
| [Development process](docs/development-process.md) | Branch → PR → CI |

Template: `plugins/example-connector`.

## Documentation

- [Container workflow](docs/container-workflow.md)
- [Development](docs/development.md)
- [Architecture](docs/architecture.md)
- [Operations](docs/operations.md)
- [Contributing](CONTRIBUTING.md)

## License

MIT OR Apache-2.0
