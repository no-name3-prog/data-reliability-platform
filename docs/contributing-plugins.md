# Contributing plugins

This guide is for engineers adding **connectors**, **validation rules**, **anomaly detectors**, **notification channels**, or **AI providers**.

## Prerequisites (host)

- Git  
- Docker CLI + Compose  

No Rust toolchain on the host. See [container-workflow.md](./container-workflow.md).

## First-time setup

```bash
git clone https://github.com/no-name3-prog/data-reliability-platform.git
cd data-reliability-platform
make doctor
make bootstrap
make verify
```

## Workflow overview

```text
branch → implement plugin crate → register one line in drp-api →
make lint && make test → open PR → CI (Docker) must pass
```

Direct pushes to `main` may be allowed depending on GitHub settings; **prefer PRs** so CI runs the same container gate as `make verify`.

## Step-by-step: new connector

1. **Copy the template**

   ```bash
   cp -R plugins/example-connector plugins/my-source
   ```

2. **Rename** the package in `plugins/my-source/Cargo.toml`  
   (e.g. `drp-plugin-my-source`).

3. **Add workspace membership** in root `Cargo.toml`:

   ```toml
   members = [
     # ...
     "plugins/my-source",
   ]

   [workspace.dependencies]
   drp-plugin-my-source = { path = "plugins/my-source", version = "0.1.0" }
   ```

4. **Implement** `ConnectorPlugin` + `Plugin` (keep `register` public).

5. **Wire composition root** — `crates/drp-api/Cargo.toml` + `register_all_plugins` in `app.rs`:

   ```rust
   drp_plugin_my_source::register(reg);
   ```

6. **Tests** (in the plugin crate or via harness):

   ```bash
   ./scripts/cargo.sh test -p drp-plugin-my-source
   make test-integration
   ```

7. **Lint / full gate**

   ```bash
   make lint
   make verify
   ```

8. **PR description** should include:
   - Plugin id(s)
   - Capability
   - Config / credentials shape
   - Test coverage summary

## Step-by-step: validation rule

1. Prefer implementing in a **new crate** `plugins/my-rule` (or extend `drp-validation` only for universal built-ins).
2. Implement `ValidatorPlugin`.
3. Register with `registry.register_validator(Arc::new(...))`.
4. Exercise via `CheckDefinition { validator: "my_rule", params: {...} }`.

## Step-by-step: anomaly detector

1. Implement `AnomalyDetectorPlugin` in `plugins/my-detector` or `drp-anomaly` (built-ins only).
2. Register with `register_anomaly_detector`.
3. Call path: `AnomalyService::detect` / `POST /v1/assets/{id}/anomalies/detect`.

## Step-by-step: notification provider

1. Implement `NotificationPlugin`.
2. Register with `register_notification`.
3. Add channel id to `notifications.default_channels` when appropriate.

## Step-by-step: AI provider

1. Implement `AiProviderPlugin` (network I/O allowed inside the plugin crate).
2. Register with `register_ai_provider`.
3. Keep secrets in env / compose — inject at construction in `register` if needed.
4. Default offline provider remains `echo` for CI without API keys.

## Definition of done

- [ ] Trait implementation is `Send + Sync`
- [ ] Stable unique plugin `id`
- [ ] `register(&PluginRegistry)` exported
- [ ] Unit tests pass in Docker
- [ ] `make lint` clean
- [ ] Composition root updated (single registration line)
- [ ] Docs: plugin id + params described (crate-level README or `docs/`)
- [ ] No changes to feature service orchestration unless required for a new capability *category*

## New capability categories

If you need a **new trait** (not just a new implementation):

1. Open a design discussion / PR against `docs/plugin-architecture.md`.
2. Add the trait + registry maps in `drp-core` only after review.
3. That is a **core** change; routine plugins must not require it.

## Container commands cheat sheet

| Task | Command |
|------|---------|
| Format | `make fmt` |
| Lint | `make lint` |
| Unit tests | `make test-unit` |
| All tests | `make test` |
| Full CI mirror | `make verify` |
| Shell in toolchain | `make shell` |
| Cargo for one package | `./scripts/cargo.sh test -p my-crate` |
