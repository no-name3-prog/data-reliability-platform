# Validation

## In plain English

**Validation** means: run quality **rules** on your data
(for example “this column must not be empty” or “values must match a pattern”).

You can run rules now or **on a schedule**. Every run is stored in **history**.
Failed checks can open **incidents**.

Use the **Validation** page in the dashboard, or the API under `/v1/checks`.

Optional: the **AI layer** can **suggest** rules from schema + profiling + samples.
Those suggestions stay **pending** until you approve them. See [AI rule suggestions](./ai-rule-suggestions.md).

---

## Overview

The validation engine runs **data quality rules** against sampled rows, on demand or on a **schedule**. Every check execution and every suite run is **appended to history** (never overwritten) so outcomes can be audited over time.

Built-in rules:

| Plugin id | Description | Key params |
|-----------|-------------|------------|
| `not_null` | Column has no nulls | `column` |
| `unique` | Column values are unique | `column` |
| `accepted_values` | Values ⊆ allowed set | `column`, `values` (array) |
| `regex` | Values match pattern | `column`, `pattern` |
| `range` | Numeric bounds | `column`, `min?`, `max?` |
| `freshness` | Data recency | `max_age_secs`, `column?` |
| `row_count` | Sample size bounds | `min?`, `max?` |
| `referential_integrity` | FK-like membership | `column`, `values` **or** `reference_asset_id` + `reference_column` |

## Architecture

```text
CheckDefinition  ──►  RuleEngine  ──►  ValidatorPlugin (by id)
                           │
ValidationService          │ sample_rows via ConnectorPlugin
     │                     ▼
     ├─ save_check_result (per-check history)
     └─ save_validation_run (suite history)

Scheduler job kind "validation"
     └─ ValidationJobHandler ──► ValidationService::run_suite
```

### Extensibility (add a new rule)

1. Implement `ValidatorPlugin` + `Plugin` (see `drp-validation` built-ins or a `plugins/*` crate).
2. Use `drp_validation::params` helpers for consistent param parsing.
3. Register:
   - Built-in: add to `register_builtin_validators`, **or**
   - External: `registry.register_validator(Arc::new(...))` in the API composition root only.
4. Create checks with `validator: "your_rule_id"` and rule-specific `params`.

No changes to `ValidationService` or core traits are required for new rule *implementations*.

## Scheduling

Create a job with kind `validation`:

```json
{
  "name": "hourly orders DQ",
  "kind": "validation",
  "schedule": "0 * * * *",
  "params": {
    "asset_id": "<ulid>",
    "connector_id": "mock",
    "check_ids": null
  }
}
```

Or use the helpers:

- `POST /v1/validation/schedule`
- Create a check with `"schedule": "0 * * * *"` (links a job automatically)

The scheduler tick loop picks up jobs with a `schedule` set and runs them via `ValidationJobHandler`.

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/validation/rules` | List registered rule plugins |
| POST | `/v1/checks` | Create check definition |
| GET | `/v1/checks` | List checks (`?asset_id=`) |
| GET | `/v1/checks/{id}` | Get check |
| POST | `/v1/checks/{id}/run` | Run one check |
| GET | `/v1/checks/{id}/results` | Per-check result history |
| POST | `/v1/assets/{id}/validate` | Run all checks for asset (suite) |
| POST | `/v1/validation/runs` | Run suite (asset and/or check_ids) |
| GET | `/v1/validation/runs` | Suite history (`?asset_id=&limit=`) |
| GET | `/v1/validation/runs/{id}` | Get suite run |
| POST | `/v1/validation/schedule` | Create scheduled validation job |

## Configuration

```toml
[validation]
fail_fast = false
default_severity = "error"

[scheduler]
enabled = true
tick_interval_secs = 5
max_concurrent_jobs = 8
```

## Example check payloads

```json
{ "name": "email required", "asset_id": "...", "validator": "not_null", "params": { "column": "email" } }
```

```json
{ "name": "status enum", "asset_id": "...", "validator": "accepted_values",
  "params": { "column": "status", "values": ["open", "closed"] } }
```

```json
{ "name": "amount range", "asset_id": "...", "validator": "range",
  "params": { "column": "amount", "min": 0, "max": 1e6 } }
```

```json
{ "name": "orders fresh", "asset_id": "...", "validator": "freshness",
  "params": { "column": "updated_at", "max_age_secs": 86400 } }
```

```json
{ "name": "fk user", "asset_id": "...", "validator": "referential_integrity",
  "params": {
    "column": "user_id",
    "reference_asset_id": "<users asset id>",
    "reference_column": "id"
  } }
```
