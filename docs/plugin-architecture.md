# Plugin architecture

The Data Reliability Platform is built so **connectors**, **validation rules**, **anomaly detectors**, **notification providers**, and **AI providers** are independent plugins. You add them **without modifying the core engine**.

## Goals

| Goal | How |
|------|-----|
| Independent delivery | New crates implement traits; core stays stable |
| Minimal core changes | Only the composition root gains a `register(...)` line |
| Type-safe extension | Rust traits + `Arc<dyn Trait>` registries |
| Testable offline | Mock / fixture / echo plugins ship in-tree |
| Containerized workflow | Same `make test` / `make lint` for plugins |

## Mental model

```text
┌─────────────────────────────────────────────────────────────┐
│  drp-api  (composition root)                                │
│    register_all_plugins(&platform.plugins)                  │
│    • one call per plugin crate — the ONLY place that knows  │
│      concrete plugin types                                  │
└───────────────────────────┬─────────────────────────────────┘
                            │ PluginRegistry (by id)
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
 feature services     feature services    feature services
 (metadata, validation, anomaly, notifications, ai, …)
        │                   │                   │
        │  resolve by id    │                   │
        ▼                   ▼                   ▼
   dyn ConnectorPlugin  dyn ValidatorPlugin  dyn AiProviderPlugin
   dyn ProfilerPlugin   dyn AnomalyDetector  dyn NotificationPlugin
```

**Rules**

1. **Traits live in `drp-core`** — the stable ABI.
2. **Implementations live outside `drp-core`** — sibling crates or `plugins/*`.
3. **Feature services depend only on traits + registry**, never on concrete plugins.
4. **Registration is explicit** at process start (no global inventory magic required).
5. **Plugin ids are stable strings** used in APIs, jobs, and configs.

## Extension-point traits

All traits are in `drp_core::plugin` and re-exported from `drp_core`.

| Trait | Capability enum | Register method | Resolve method |
|-------|-----------------|-----------------|----------------|
| `ConnectorPlugin` | `Connector` | `register_connector` | `connector(id)` |
| `ProfilerPlugin` | `Profiler` | `register_profiler` | `profiler(id)` |
| `ValidatorPlugin` | `Validator` | `register_validator` | `validator(id)` |
| `AnomalyDetectorPlugin` | `AnomalyDetector` | `register_anomaly_detector` | `anomaly_detector(id)` |
| `NotificationPlugin` | `Notification` | `register_notification` | `notification(id)` |
| `AiProviderPlugin` | `AiProvider` | `register_ai_provider` | `ai_provider(id)` |

Shared base:

- `Plugin` — `fn info(&self) -> &PluginInfo`
- `PluginInfo` — `id`, `name`, `version`, `description`, `capabilities`
- `PluginContext` — free-form JSON config + optional tenant
- `PluginBundle` — optional multi-plugin register helper

### ConnectorPlugin

```rust
async fn test_connection(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<()>;
async fn discover(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<Vec<Asset>>;
async fn sample_rows(&self, asset: &Asset, limit: usize, ctx: &PluginContext)
    -> Result<Vec<IndexMap<String, Value>>>;
```

### ValidatorPlugin (validation rules)

```rust
async fn validate(
    &self,
    check: &CheckDefinition,  // includes params map
    asset: &Asset,
    rows: &[IndexMap<String, Value>],
    ctx: &PluginContext,
) -> Result<CheckResult>;
```

### AnomalyDetectorPlugin

```rust
async fn detect(
    &self,
    asset: &Asset,
    rows: &[IndexMap<String, Value>],
    ctx: &PluginContext,
) -> Result<AnomalyReport>;
```

### NotificationPlugin

```rust
async fn send(
    &self,
    subject: &str,
    body: &str,
    metadata: &IndexMap<String, Value>,
    ctx: &PluginContext,
) -> Result<()>;
```

### AiProviderPlugin

```rust
async fn complete(&self, request: &AiRequest, ctx: &PluginContext) -> Result<AiResponse>;
async fn health(&self, ctx: &PluginContext) -> Result<()>; // default OK
```

## Built-in plugins (reference)

| Id | Crate | Kind |
|----|-------|------|
| `mock`, `fixture` | `drp-connectors` | Connector |
| `example` | `plugins/example-connector` | Connector (template) |
| `basic` | `drp-profiling` | Profiler |
| `not_null`, `unique`, `regex` | `drp-validation` | Validator |
| `null_spike`, `zscore` | `drp-anomaly` | Anomaly detector |
| `log` | `drp-notifications` | Notification |
| `echo` | `drp-ai` | AI provider |

## Adding a plugin (no core changes)

### Checklist

1. **Copy** `plugins/example-connector` (or scaffold a new crate under `plugins/` or `crates/`).
2. **Depend** only on `drp-core` + `drp-common` (+ minimal deps you need).
3. **Implement** the trait + `Plugin`.
4. **Export** `pub fn register(registry: &PluginRegistry)`.
5. **Add** the crate to the workspace `[workspace.members]` (if in-repo).
6. **Call** `your_crate::register(&platform.plugins)` inside `register_all_plugins` in `crates/drp-api/src/app.rs`.
7. **Test** with `make test-unit` / `make test` (Docker).
8. **Document** the plugin id and config keys in your crate README.

### What you must not do

- Edit `drp-core` traits unless proposing a new *category* of extension (RFC / design review).
- Import concrete plugin types from metadata/validation/anomaly services.
- Require host-side `cargo` — always use container Make targets.

### Minimal registration patch (composition root only)

```rust
// crates/drp-api/src/app.rs — inside register_all_plugins
my_org_drp_connector_snowflake::register(reg);
```

That is the only platform file that should change for a typical plugin.

## Optional: PluginBundle

For crates that ship many plugins:

```rust
pub struct AcmeBundle;

impl PluginBundle for AcmeBundle {
    fn register(&self, registry: &PluginRegistry) {
        registry.register_connector(Arc::new(AcmeDb::new()));
        registry.register_validator(Arc::new(AcmeRule::new()));
    }
}
```

## Dependency rules

```text
drp-common          (no plugins)
    ▲
drp-core            (traits + registry + domain types ONLY)
    ▲
plugin crates       (connectors, validation, anomaly, ai, notifications, plugins/*)
    ▲
feature services    (metadata, profiling, …)  — use registry, not concrete plugins
    ▲
drp-api             (composition root: register + HTTP)
```

**Forbidden edges**

- `drp-core` → any plugin crate  
- `drp-metadata` → `drp-connectors` concrete types (registry only; tests may use mocks via `drp-test-support`)

## Configuration

Plugins receive:

1. **Definition-level params** (e.g. `CheckDefinition.params` for validators).
2. **Invocation context** (`PluginContext.config`) for run-time options.
3. **Env / AppConfig** only at composition time when constructing the plugin (`MyPlugin::from_config(&cfg)`).

Prefer constructor injection over reading process env inside `detect`/`validate`.

## Versioning

- Plugin **ids** are part of the public API surface — do not rename lightly.
- Plugin **crate versions** follow workspace semver; breaking trait changes require a major `drp-core` bump.
- Advertise capabilities accurately on `PluginInfo` for discovery via `GET /v1/plugins`.

## Testing plugins

| Approach | Tooling |
|----------|---------|
| Unit-test the plugin in isolation | `#[tokio::test]` in the plugin crate |
| Integration with platform harness | `drp-test-support::PlatformHarness` + register your plugin |
| Regression / golden data | `FixtureConnector` + fixtures under `crates/drp-tests` |

Always:

```bash
make test-unit
make test
make lint
```

See [testing.md](./testing.md) and [contributing-plugins.md](./contributing-plugins.md).
