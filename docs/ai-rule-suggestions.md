# AI rule suggestions

## In plain English

The platform can **suggest** validation rules by looking at:

- the dataset **schema** (column names and types)
- the latest **profile** (null rates, uniqueness, semantic types)
- a few **sample rows**

Suggestions are **not active** until a person **approves** them.
Approving creates a normal validation check you can run on a schedule.

You can use a built-in offline engine (`heuristic`) with no API key, or plug in
any OpenAI-compatible model (SpaceXAI/xAI, Ollama, etc.).

---

## Flow

```text
schema + profile + samples
        │
        ▼
  AI provider (pluggable)
        │
        ▼
 RuleSuggestion (status = pending)   ◄── review queue
        │
   ┌────┴────┐
approve    reject
   │           │
   ▼           ▼
CheckDefinition   (closed)
(enabled=true)
```

## API

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/v1/ai/status` | Whether AI is enabled + default provider |
| `GET` | `/v1/ai/providers` | Registered AI provider plugins |
| `POST` | `/v1/assets/{id}/ai/suggest-rules` | Generate pending suggestions |
| `GET` | `/v1/ai/suggestions` | List (`?asset_id=&status=pending&limit=`) |
| `GET` | `/v1/ai/suggestions/{id}` | Get one |
| `POST` | `/v1/ai/suggestions/{id}/approve` | Activate as a check |
| `POST` | `/v1/ai/suggestions/{id}/reject` | Discard |

### Suggest body

```json
{
  "connector": "mock",
  "provider": "heuristic"
}
```

`provider` is optional (defaults to config `ai.default_provider`).

### Approve / reject body (optional)

```json
{ "reviewed_by": "alice", "reason": "too noisy" }
```

## Pluggable AI providers

Providers implement `AiProviderPlugin` in `drp-core` and register at process start.

| Plugin id | Network | Role |
|-----------|---------|------|
| `heuristic` | No | Deterministic rules from schema/profile (default) |
| `echo` | No | Stub completions for tests |
| `openai_compatible` | Yes | SpaceXAI/xAI, Ollama, OpenAI, any `/v1/chat/completions` server |

### Config (`DRP_AI__*`)

```yaml
# config/default.yaml (or env)
ai:
  enabled: true
  default_provider: heuristic
  sample_rows: 20
  openai_compatible:
    enabled: false
    base_url: https://api.x.ai/v1
    api_key_env: XAI_API_KEY
    model: grok-4.5
```

Environment examples:

```bash
# Enable SpaceXAI / xAI
export XAI_API_KEY=...
export DRP_AI__OPENAI_COMPATIBLE__ENABLED=true
export DRP_AI__DEFAULT_PROVIDER=openai_compatible

# Local Ollama
export DRP_AI__OPENAI_COMPATIBLE__ENABLED=true
export DRP_AI__OPENAI_COMPATIBLE__BASE_URL=http://host.docker.internal:11434/v1
export DRP_AI__OPENAI_COMPATIBLE__API_KEY_ENV=   # empty = no auth
export DRP_AI__OPENAI_COMPATIBLE__MODEL=llama3.2
export DRP_AI__DEFAULT_PROVIDER=openai_compatible
```

When a remote provider fails or returns unparseable JSON, the service **falls back**
to the offline heuristic engine so suggestions still appear for review.

### Adding a custom provider

1. Implement `AiProviderPlugin` in a crate.
2. Call `registry.register_ai_provider(Arc::new(...))` from `drp-api` composition root
   (same pattern as connectors).
3. Pass `provider: "your_id"` on suggest, or set `ai.default_provider`.

## Dashboard

On the **Validation** page:

1. Discover mock (or real) datasets.
2. Click **Suggest rules (AI)**.
3. Review the pending list → **Approve** or **Reject**.
4. Approved rules appear under **Checks** and can be run.

## Safety

- Suggestions never run as checks until approved.
- Rejected suggestions never create checks.
- `ai.enabled=false` disables the suggest API.
