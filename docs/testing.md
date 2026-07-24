# Testing infrastructure

All tests run **inside Docker** via the `dev` toolchain image. Do not run
`cargo test` / `cargo nextest` on the host.

## Layers

| Layer | What | How to run |
|-------|------|------------|
| **Unit** | Per-crate pure logic, mock connectors | `make test-unit` |
| **Integration** | Multi-crate flows + HTTP (in-process axum) | `make test-integration` |
| **Regression** | Golden fixtures / stable DQ expectations | `make test-regression` |
| **Full CI suite** | nextest `--profile ci` | `make test` |
| **Local CI mirror** | lint + all profiles + build + doc | `make verify` |

## Tooling

- **cargo-nextest** `0.9.100` (pinned for rustc 1.85) installed in `docker/Dockerfile.dev`
- Config: `.config/nextest.toml` (profiles: `unit`, `integration`, `regression`, `ci`)
- Shared harness: `crates/drp-test-support`
- Integration/regression package: `crates/drp-tests`
- Fixtures: `crates/drp-tests/fixtures/`

## Mock connectors

| Plugin id | Type | Purpose |
|-----------|------|---------|
| `mock` | `MockConnector` | Fixed orders/users sample |
| `fixture` | `FixtureConnector` | Configurable tables for tests |
| `failing` | `FailingConnector` | Negative-path (register in test) |

## Naming conventions

- Integration tests live in `crates/drp-tests/tests/integration_*.rs`
- Regression tests live in `crates/drp-tests/tests/regression_*.rs` and use function names containing `regression_`
- Unit tests live in each crate’s `src/**` or `tests/` (package ≠ `drp-tests`)

## CI parity

GitHub Actions (`.github/workflows/ci.yml`) runs the same Make targets as local
`make verify`, then builds the production API image and probes `/readyz`,
`/livez`, and `/metrics` on the compose network.
