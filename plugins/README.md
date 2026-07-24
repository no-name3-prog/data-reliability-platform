# Plugins

Optional and example plugin crates that **do not belong in core**.

| Path | Description |
|------|-------------|
| `example-connector/` | Template connector — **copy this** to start a new source plugin |

## Rules

1. Depend on `drp-core` / `drp-common`, not on `drp-api`.
2. Export `pub fn register(registry: &PluginRegistry)`.
3. Register from `drp-api` composition root only.
4. Develop with `make test` / `make lint` (Docker).

See [docs/plugin-architecture.md](../docs/plugin-architecture.md) and
[docs/contributing-plugins.md](../docs/contributing-plugins.md).
