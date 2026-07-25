# Repository structure

## In plain English

This is a map of the folders.  
You do not need to memorize it to run the demo — only when you change code.

| Folder | Meaning |
|--------|---------|
| `crates/` | Backend pieces |
| `web/` | Dashboard UI |
| `docs/` | Guides |
| `docker/` | How we package and run things |
| `config/` | Settings files |
| `plugins/` | Example add-ons |

---

## Where new code goes

| You are adding… | Put it in… | Touch core? |
|-----------------|------------|-------------|
| Connector / source | `plugins/<name>` or `crates/drp-connector-*` | No (only `drp-api` register) |
| Validation rule | `plugins/<name>` or `drp-validation` (universal only) | No |
| Anomaly detector | `plugins/<name>` or `drp-anomaly` | No |
| Notification channel | `plugins/<name>` or `drp-notifications` | No |
| AI provider | `plugins/<name>` or `drp-ai` | No |
| New plugin *category* | `drp-core` traits + registry | **Yes** (design review) |
| HTTP route | `drp-api` | No (unless new domain) |
| Cross-cutting type | `drp-common` or `drp-core::domain` | Careful / review |

## Composition root

`crates/drp-api/src/app.rs` owns `register_all_plugins`. This is the **single chokepoint** that knows concrete plugin crates. Feature services remain plugin-agnostic.
