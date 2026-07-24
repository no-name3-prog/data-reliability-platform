# Data Reliability Platform

**Container-first** Rust monorepo for data reliability: catalog metadata, profiling, validation, lineage, scheduling, and notifications.

## Host prerequisites (only these)

| Tool | Purpose |
|------|---------|
| **Git** | Clone / version control |
| **Docker CLI** + **Compose** | Build, lint, test, run, docs |

### Explicitly not required on the host

- Rust toolchain (`rustc`, `cargo`, `rustup`)
- PostgreSQL client or server
- Redis
- MinIO / AWS CLI
- Running tests with host `cargo test`

Everything runs **inside Docker containers**.

```text
┌─────────────────────────────────────────────────────────┐
│  Host machine                                           │
│    git  ·  docker  ·  docker-compose                    │
│         │                                               │
│         ▼                                               │
│  ┌──────────── compose network ──────────────────────┐  │
│  │  dev (Rust toolchain)  api  postgres  redis  minio│  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Quick start

```bash
git clone <repo-url> data-reliability-platform
cd data-reliability-platform

# Verify host has Docker only (and warn if cargo/psql leak onto PATH)
make doctor

# Build the toolchain image + start Postgres, Redis, MinIO
make bootstrap

# Build, lint, and test — all inside containers
make build
make lint
make test

# Run the API (production-like image) with infra
make up
curl -s http://127.0.0.1:8080/health
```

## Everyday commands

| Command | What runs where |
|---------|-----------------|
| `make doctor` | Host checks only (Docker presence) |
| `make bootstrap` | Compose: build `dev` image, start infra |
| `make build` | Container: `cargo build --workspace` |
| `make test` | Container: `cargo test --workspace` |
| `make lint` | Container: `fmt --check` + `clippy -D warnings` |
| `make fmt` | Container: `cargo fmt` |
| `make doc` | Container: `cargo doc` |
| `make docs-serve` | Container: nginx serving rustdoc on `:3001` |
| `make shell` | Interactive bash **inside** the Rust image |
| `make up` | Compose: infra + API |
| `make down` | Stop containers |
| `make clean` | Remove containers + named volumes |

Arbitrary cargo (still containerized):

```bash
./scripts/cargo.sh test -p drp-core
./scripts/cargo.sh clippy -p drp-api
```

## Workspace crates

| Crate | Role |
|-------|------|
| `drp-common` | Errors, IDs, config, shared types |
| `drp-core` | Domain model, plugins, event bus |
| `drp-storage` | `Store` trait + memory backend |
| `drp-connectors` | Connector plugins (`mock`) |
| `drp-metadata` | Asset catalog |
| `drp-profiling` | Profiling engine |
| `drp-validation` | DQ checks (`not_null`, `unique`, `regex`) |
| `drp-lineage` | Lineage graph |
| `drp-scheduler` | Jobs / handlers |
| `drp-notifications` | Alert channels (`log`) |
| `drp-api` | Axum HTTP API + `drp` binary |

See [docs/architecture.md](docs/architecture.md) and [docs/container-workflow.md](docs/container-workflow.md).

## Infrastructure (compose only)

| Service | Image | Port (host) |
|---------|-------|-------------|
| `postgres` | `postgres:16-alpine` | 5432 |
| `redis` | `redis:7-alpine` | 6379 |
| `minio` | `minio/minio` | 9000 / 9001 |
| `api` | multi-stage `docker/Dockerfile` | 8080 |
| `dev` | `docker/Dockerfile.dev` | (toolchain) |

Inside the compose network, apps use DNS names `postgres`, `redis`, `minio` — never `localhost` for cross-service traffic.

## Configuration

- Defaults: `config/default.toml`
- Env sample: `.env.example` → copy to `.env` for compose substitution
- Infra URLs default to compose service names

## Extending (plugins)

Implement `ConnectorPlugin` / `ValidatorPlugin` / … and register at composition root in `drp-api`. No host toolchain needed: edit sources on the host, run `make build` / `make test` in containers.

## CI

GitHub Actions uses **Docker only** (see `.github/workflows/ci.yml`). No `dtolnay/rust-toolchain` on the runner.

## License

MIT OR Apache-2.0
