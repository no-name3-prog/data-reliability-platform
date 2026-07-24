# Container-first workflow

## Contract

1. Developers install **Git + Docker CLI + Compose** only.
2. **Never** install or use a host Rust toolchain for this repository.
3. **Never** install or run PostgreSQL, Redis, MinIO, or Prometheus on the host for this project.
4. **Never** run `cargo test` / `cargo build` / `clippy` on the host PATH.
5. All build, lint, test, and docs commands enter a container via `make` or `scripts/*.sh`.
6. Git hooks (`.githooks/`) call `make` — still Docker-backed.

## Why

- Identical toolchain (Rust 1.85, clippy, rustfmt) for every contributor and CI.
- Infra parity: API always talks to compose DNS names.
- No “works on my machine” drift.

## Layout

| Path | Role |
|------|------|
| `docker/Dockerfile.dev` | Toolchain image |
| `docker/Dockerfile` | Production API image |
| `docker/Dockerfile.docs` | rustdoc → nginx |
| `docker/prometheus/` | Scrape config |
| `docker-compose.yml` | Infra + api + prometheus + dev |
| `Makefile` | Thin Docker wrappers |
| `scripts/cargo.sh` | `compose run dev cargo …` |
| `scripts/drp.sh` | Unified entrypoint |
| `.githooks/` | pre-commit / pre-push → make |

## Caches

Named volumes: `drp-cargo-registry`, `drp-cargo-git`, `drp-target` (inside Docker only).  
`make clean` removes compose volumes including caches.
