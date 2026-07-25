# Getting started

This guide is for **anyone** who wants to run the platform and try it with sample data.

## Before you begin

1. Install **Docker Desktop** (or Docker Engine + Compose) and start it.
2. Install **Git**.
3. Open a terminal.

You do **not** need Rust, Node, Postgres, or Redis on your machine.

## Step 1 — Get the code

```bash
git clone https://github.com/no-name3-prog/data-reliability-platform.git
cd data-reliability-platform
```

## Step 2 — Check Docker

```bash
make doctor
```

If this fails, start Docker and try again.

## Step 3 — First-time setup

```bash
make bootstrap
```

This builds the developer image and starts support services (Postgres, Redis, MinIO).  
The first time can take a while.

## Step 4 — Start the app

```bash
make up
```

This starts:

- the **API** (port 8080)
- the **dashboard** (port 3000)
- databases and metrics

Wait until the command finishes. First run may **build images for a long time**.

## Step 5 — Open the dashboard

In your browser:

**http://127.0.0.1:3000**

Check the API is alive:

```bash
curl -s http://127.0.0.1:8080/readyz
```

You should get a success response (not a connection error).

## Step 6 — Demo with dummy data

1. On the **Overview** page, click **Load mock data**.
2. Go to **Datasets**. You should see **orders** and **users**.
3. Open **orders**.
4. Click **Profile** (or use the **Profiling** page).
5. Go to **Validation** → **Run demo not-null check**.  
   - Order #2 has a missing email → the check should **fail**.
6. Open **Incidents** — a new incident may appear.
7. Open **Lineage** → **Ingest SQL** to build a sample graph.

### What is in the mock data?

| Table | Notes |
|-------|--------|
| **orders** | 4 rows; one row has a **null** `customer_email` |
| **users** | 2 rows with emails and timestamps |

No real database is required for this demo.

## Common problems

| Problem | What to do |
|---------|------------|
| “Cannot connect” to :3000 or :8080 | Run `make up`. Wait for build. Check `docker-compose ps`. |
| Only Postgres/Redis show, no `api`/`web` | Build still running or failed. Check `make logs`. |
| Dashboard says Offline | API not ready. Wait, then refresh. |
| Docker errors | Start Docker Desktop / Colima, then `make doctor`. |
| Port already in use | Stop the other app, or set `DRP_WEB_PORT=3001` / `DRP_API_PORT=8081`. |
| Data disappeared after restart | Default storage is **memory**. Use Postgres for permanent data (see config). |

## Stop the app

```bash
make down
```

## Next steps

- [Dashboard guide](dashboard.md) — what each page does  
- [Testing](testing.md) — run automated tests  
- [Contributing](../CONTRIBUTING.md) — how to send a pull request  
- Feature docs: [profiling](profiling.md), [validation](validation.md), [lineage](lineage.md), [incidents](incidents.md)

## Talk to the API (optional)

If you prefer `curl` instead of the UI:

```bash
# Discover mock tables
curl -s -X POST http://127.0.0.1:8080/v1/assets/discover \
  -H 'content-type: application/json' \
  -d '{"connector":"mock","uri":"mock://local"}'
```

See each feature doc for more API examples.
