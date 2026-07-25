# Anomaly detection engine

## Overview

The anomaly engine compares the **latest dataset profile** with **historical profiles** and raises **incidents** when unusual changes appear:

| Kind | Signal |
|------|--------|
| `schema_change` | Columns added/removed or data type changed |
| `row_count_drop` | Row count falls by ≥ configured ratio |
| `null_spike` | Null % rises by ≥ configured points |
| `duplicate_spike` | Unique ratio drops (more duplicates) |
| `distribution_change` | Numeric mean shift beyond z-threshold |
| `freshness` | Profile (or date column max) older than SLA |

Sample-based plugins (`null_spike`, `zscore`) remain available for row-level detection.

## Architecture

```text
Profile history (Store)
        │
        ▼
ProfileAnomalyEngine  (rules: schema, row_count, null, duplicate, distribution, freshness)
        │
        ▼
AnomalyReport  ──►  Store (append)
        │
        └─► Incident per finding  ──►  Store + PlatformEvent::IncidentOpened
```

### Extending

Implement `ProfileAnomalyRule` and register in `ProfileAnomalyEngine::with_defaults`, **or** implement `AnomalyDetectorPlugin` for sample-based detectors and register via `PluginRegistry`.

## API

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/assets/{id}/anomalies/analyze` | Profile-history analysis (primary) |
| POST | `/v1/assets/{id}/anomalies/detect` | Sample detector plugin |
| GET | `/v1/assets/{id}/anomalies/reports` | Report history |
| GET | `/v1/anomaly-reports/{run_id}` | One report |
| GET | `/v1/incidents?asset_id=&limit=` | List incidents |
| GET | `/v1/incidents/{id}` | Get incident |
| PATCH | `/v1/incidents/{id}` | Update status (`open` / `acknowledged` / `resolved`) |

Typical flow: **discover → profile → analyze**.

## Configuration

```toml
[anomaly]
history_window = 10
row_count_drop_ratio = 0.3
null_spike_delta = 10.0
duplicate_unique_ratio_drop = 0.2
distribution_zscore = 3.0
freshness_max_age_secs = 86400
create_incidents = true
```
