# Container-first workflow

## Contract

1. Developers install **Git + Docker CLI + Compose** only.
2. **Never** install or use a host Rust toolchain for this repository.
3. **Never** install or run PostgreSQL, Redis, or MinIO on the host for this project.
4. **Never** run `cargo test` / `cargo build` / `clippy` on the host PATH.
5. All build, lint, test, and docs commands enter a container via `make` or `scripts/cargo.sh`.

## Why

- Identical toolchain (Rust 1.85, clippy, rustfmt) for every contributor and CI.
- Infra parity: API always talks to compose DNS names.
- No “works on my machine” drift from local postgres versions or cargo installs.

## Layout

| Path | Role |
|------|------|
| `docker/Dockerfile.dev` | Toolchain image (`rust:1.85` + clippy/rustfmt) |
| `docker/Dockerfile` | Multi-stage production API image |
| `docker/Dockerfile.docs` | rustdoc → nginx |
| `docker-compose.yml` | Infra + api + one-shot tool services |
| `Makefile` | Thin Docker wrappers (no host cargo) |
| `scripts/cargo.sh` | `docker-compose run --rm dev cargo …` |
| `scripts/guard-host.sh` | Optional guard against host cargo |

## Caches

Compose mounts named volumes so cargo registry and `target/` persist **inside Docker volumes**, not as a host-managed toolchain:

- `drp-cargo-registry`
- `drp-cargo-git`
- `drp-target`

`make clean` removes these volumes.

## Adding a dependency

1. Edit `Cargo.toml` / crate manifests on the host (text editor only).
2. `make build` or `./scripts/cargo.sh update` — resolution happens in the container.
3. Commit `Cargo.lock` when present after containerized builds.

## Debugging

```bash
make shell          # bash inside toolchain image
make logs           # api + infra logs
make doctor         # host prerequisite check
```
