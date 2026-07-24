# Contributing

## Prerequisites (host only)

- **Git**
- **Docker CLI** + **Compose**

Do **not** install Rust, PostgreSQL, Redis, MinIO, or Prometheus on the host.

## First-time setup

```bash
git clone <repo-url>
cd data-reliability-platform
make doctor
make bootstrap   # toolchain image + infra + git hooks
make build
make test
make up          # api + prometheus + infra
```

## Everyday workflow

| Task | Command |
|------|---------|
| Format | `make fmt` |
| Lint | `make lint` |
| Test | `make test` |
| Full gate | `make check` or `make ci` |
| Shell in toolchain | `make shell` |
| Arbitrary cargo | `./scripts/cargo.sh test -p drp-core` |
| Unified helper | `./scripts/drp.sh test` |

All of the above execute **inside containers**.

## Git hooks

`make bootstrap` (or `make hooks`) sets `core.hooksPath=.githooks`:

- **pre-commit** → `make fmt-check` + `make clippy` (Docker)
- **pre-push** → `make test` (Docker)

Optional: if you use the Python `pre-commit` tool, `.pre-commit-config.yaml` also shells out to `make` (still containerized).

## Pull requests

1. Branch from `main`
2. `make ci` must pass locally (containerized)
3. CI on GitHub runs the same Docker gate — no host Rust on runners

## Code standards

- `rustfmt` + `clippy -D warnings` (see `rustfmt.toml`, `clippy.toml`)
- EditorConfig (`.editorconfig`)
- No `unsafe` in platform crates (`forbid(unsafe_code)`)
- Prefer plugins over forking core services (see `docs/architecture.md`)
