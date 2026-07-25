# Development

## Ground rules

**OK on your laptop:** Git, Docker, editor, `make`  
**Not needed on your laptop:** Rust, local Postgres, host `cargo test`

## Start coding

```bash
make doctor
make bootstrap
make shell          # optional: shell inside the build container
```

Run the full product while you work:

```bash
make up
# UI  http://127.0.0.1:3000
# API http://127.0.0.1:8080
```

## Check your change

```bash
make fmt      # auto-format
make lint     # format check + clippy
make test
```

## Services when `make up` is running

| Service | Port | Purpose |
|---------|------|---------|
| Dashboard (`web`) | 3000 | UI |
| API | 8080 | HTTP API |
| Postgres | 5432 | Database (optional backend) |
| Redis | 6379 | For future features |
| MinIO | 9000 / 9001 | File storage |
| Prometheus | 9090 | Metrics UI |

## Health URLs

| URL | Meaning |
|-----|---------|
| `/livez` | Process is up |
| `/readyz` | Ready to take traffic |
| `/metrics` | Prometheus metrics |

## Logs and config

- Settings: `config/default.toml`
- Env examples: `DRP_LOG_LEVEL`, `DRP_STORAGE_BACKEND`, notification webhooks  
- Logs: `make logs` or `docker-compose logs -f api`

## Docs for developers

```bash
make doc          # generate API docs inside Docker
```
