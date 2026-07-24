# Development process

## Principles

1. **Container-first** — build, lint, test, docs run in Docker (`make *`).
2. **Plugin-first features** — prefer a new plugin over forking services.
3. **Small PRs** — one plugin or one vertical slice per PR when possible.
4. **CI parity** — local `make verify` ≈ GitHub Actions quality job.

## Lifecycle of a change

```text
1. Issue / goal
2. Choose layer: plugin vs service vs core trait
3. Branch from main
4. Implement + unit tests
5. make lint && make test (or make verify)
6. Pull request
7. CI green (Docker)
8. Review → merge
```

## Choosing the right layer

| Need | Approach |
|------|----------|
| Support a new warehouse / API | `ConnectorPlugin` crate |
| New DQ assertion | `ValidatorPlugin` |
| New statistical / ML detector | `AnomalyDetectorPlugin` |
| Slack / PagerDuty / email | `NotificationPlugin` |
| LLM summarization / RCA assist | `AiProviderPlugin` |
| Change how jobs are scheduled | `drp-scheduler` (+ maybe `JobHandler`) |
| Change shared domain model | `drp-core` / `drp-common` (review) |

## Daily commands

```bash
make shell              # toolchain container
make fmt && make lint
make test-unit
make test-integration
make test-regression
make test               # nextest ci profile
make verify             # full gate
./scripts/cargo.sh test -p drp-anomaly
```

## Code review expectations

- Plugins: clear id, docs, tests, no core churn  
- Services: depend on registry, not concrete plugins  
- API: versioned routes under `/v1`, consistent errors  
- No secrets in git; use compose env / `.env` (gitignored)

## Release notes (plugins)

When shipping a plugin, list:

- Plugin id(s)  
- Capability  
- Config keys  
- Breaking changes (id renames are breaking)
