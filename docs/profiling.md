# Profiling engine

## Overview

After dataset **discovery**, the platform can **automatically profile** each asset:

- Row count (sample)
- Null count / **null percentage**
- **Unique** (distinct) values and unique ratio
- **Min / max / average** (and stddev) for numerics
- **Histograms** (equal-width numeric or top categorical)
- **Semantic types**: email, phone, date, datetime, url, uuid, ip, category, text, …

Every profile run is **appended to history** (never overwrites), so changes can be compared over time.

## Architecture

```text
Discovery (metadata)
    │
    ▼
ProfilingService::profile_assets_batch  (auto after POST /v1/assets/discover)
    │
    ├─ ConnectorPlugin::sample_rows
    ├─ ProfilerPlugin ("basic")  → DatasetProfile
    └─ Store::save_profile       → history (memory or postgres)
```

## API

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/assets/{id}/profile` | Run profiler now |
| GET | `/v1/assets/{id}/profile` | Latest profile |
| GET | `/v1/assets/{id}/profiles?limit=N` | History (newest first) |
| GET | `/v1/assets/{id}/profiles/{run_id}` | Specific run |
| GET | `/v1/assets/{id}/profiles/compare` | Diff latest vs previous (or `?baseline=&current=`) |

Discovery:

```http
POST /v1/assets/discover
```

registers assets **and** auto-profiles each one (when auto-profile is enabled).

## Configuration

```toml
[profiling]
sample_size = 10000
null_threshold = 0.05
```

Env: `DRP_LOG_LEVEL`, storage backend for durable history (`memory` default; `postgres` when available).

## Extending

Implement `ProfilerPlugin` for alternative algorithms; register beside `basic`.
