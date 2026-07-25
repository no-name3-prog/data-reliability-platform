# Data Reliability Platform

A tool that helps you **check whether your data is healthy**.

It can:

1. **Find** your tables and files (discover data sources)
2. **Measure** them (profiling — nulls, types, stats)
3. **Test** them with rules (validation — e.g. “email must not be empty”)
4. **Spot odd changes** over time (anomalies)
5. **Show what depends on what** (lineage)
6. **Open tickets** when something fails (incidents + Slack/email/webhooks)

You use it with:

- a **web dashboard** (point and click), and/or  
- a **HTTP API** (for scripts and pipelines)

Everything runs in **Docker**. You do **not** need to install Rust or databases on your laptop.

---

## What you need on your computer

| Tool | Why |
|------|-----|
| [Git](https://git-scm.com/) | Download the code |
| [Docker](https://www.docker.com/products/docker-desktop/) | Run the app |

That’s it. Leave Docker **running** before you start.

---

## Start in 5 minutes

```bash
git clone https://github.com/no-name3-prog/data-reliability-platform.git
cd data-reliability-platform

make doctor      # checks Docker is OK
make bootstrap   # first time only — builds images, starts support services
make up          # starts API + dashboard + databases
```

The first `make up` can take **several minutes** (building images). Wait until it finishes.

### Open these links

| What | Address |
|------|---------|
| **Dashboard (UI)** | http://127.0.0.1:3000 |
| API health check | http://127.0.0.1:8080/readyz |
| Prometheus (metrics) | http://127.0.0.1:9090 |

If the health check fails, the API is not ready yet. Wait a bit, then try again.  
Or run: `docker-compose ps` and `make logs`.

### Try the demo (no real database needed)

1. Open **http://127.0.0.1:3000**
2. Click **Load mock data**
3. Open **Datasets** → click **orders**
4. Try **Profiling** and **Validation**
5. Check **Incidents** if a check fails

Mock data includes an **orders** table with one missing email — perfect for a failing quality check.

Stop everything:

```bash
make down
```

More detail: [docs/getting-started.md](docs/getting-started.md)

---

## Everyday commands

Run these from the project folder. They all use Docker under the hood.

| I want to… | Command |
|------------|---------|
| Start the full app | `make up` |
| Stop the app | `make down` |
| See if Docker is OK | `make doctor` |
| Run tests | `make test` |
| Check code style | `make lint` |
| Run everything CI runs | `make verify` |
| Open a shell inside the build container | `make shell` |
| Rebuild only the dashboard | `make web-build && make web` |

**Do not** run `cargo test` or `cargo build` on your host. Use `make` instead.

---

## How the pieces fit together

```text
Discover data  →  Profile  →  Validate  →  Anomalies
                      ↓
                 Lineage (who is affected?)
                      ↓
                 Incidents + alerts
```

| Area | What it does | Docs |
|------|----------------|------|
| Connectors | Talk to Postgres, CSV, Parquet, mock data | [connectors](docs/connectors.md) |
| Profiling | Stats, types, history | [profiling](docs/profiling.md) |
| Validation | Quality rules + schedules | [validation](docs/validation.md) |
| Anomalies | Detect unusual profile changes | [anomaly](docs/anomaly.md) |
| Lineage | SQL graph + impact | [lineage](docs/lineage.md) |
| Incidents | Severity, owner, timeline, notifications | [incidents](docs/incidents.md) |
| Dashboard | Web UI | [dashboard](docs/dashboard.md) |

---

## Project layout (simple view)

| Folder | What’s inside |
|--------|----------------|
| `crates/` | Backend (Rust) — API and engines |
| `web/` | Dashboard (React) |
| `docs/` | Guides like this one |
| `docker/` | Docker images |
| `config/` | Settings (logging, notifications, …) |
| `plugins/` | Example plugins you can copy |

You do **not** need to understand all of this to run the demo.

---

## Contributing (how we accept changes)

1. **Never push straight to `main`** — it is blocked.
2. Create a **branch**, make your change, open a **Pull Request**.
3. Wait for **GitHub Actions** (tests in Docker) to pass.
4. The **repo owner** merges the PR.

```bash
git checkout main
git pull origin main
git checkout -b feature/my-change

# edit files…
make lint
make test

git add -A
git commit -m "feat: short description of change"
git push -u origin HEAD
gh pr create --base main
```

Full guide: [CONTRIBUTING.md](CONTRIBUTING.md)

---

## Learn more

| Guide | For |
|-------|-----|
| [Getting started](docs/getting-started.md) | First run + demo walkthrough |
| [Dashboard](docs/dashboard.md) | Using the UI |
| [Testing](docs/testing.md) | How tests work |
| [Container workflow](docs/container-workflow.md) | Why everything is in Docker |
| [Development](docs/development.md) | Day-to-day coding |
| [Architecture](docs/architecture.md) | How crates fit together |
| [Operations](docs/operations.md) | Logs, health, metrics |

---

## License

MIT OR Apache-2.0
