# Incident management

## Overview

Every **failed validation** and **anomaly finding** can open an **incident** with:

| Field | Description |
|-------|-------------|
| Severity | Low / medium / high / critical |
| Status | open → in_progress → acknowledged → monitoring → resolved |
| Owner | Team or user assignee |
| Timeline | Full append-only history of events |
| Affected assets | Primary asset + optional lineage expansion |
| Source | `validation` / `anomaly` / `manual` |

Notifications fan out to **log**, **Slack**, **email**, and **webhooks** (empty URLs = dry-run logs for local/CI).

## Flow

```text
Validation failed  ──┐
                     ├──► IncidentService::open_* ──► Store + timeline
Anomaly finding   ──┘              │
                                   ├──► NotificationService (slack/email/webhook/log)
                                   └──► PlatformEvent::IncidentOpened
```

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/incidents` | List (`?asset_id=&limit=`) |
| GET | `/v1/incidents/{id}` | Get with timeline |
| GET | `/v1/incidents/{id}/history` | Complete history |
| POST | `/v1/incidents/{id}/status` | Update status |
| POST | `/v1/incidents/{id}/owner` | Assign owner |
| POST | `/v1/incidents/{id}/notes` | Add note |
| POST | `/v1/incidents/{id}/affected-assets` | Set affected assets (`include_lineage_downstream`) |

## Configuration

```toml
[notifications]
enabled = true
default_channels = ["log", "slack", "email", "webhook"]
slack_webhook_url = ""      # empty = dry-run
email_to = ""
email_webhook_url = ""
webhook_url = ""

[anomaly]
create_incidents = true
```

## Extending channels

Implement `NotificationPlugin`, register in composition root, add channel id to `default_channels`.
