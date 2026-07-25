# Container workflow (why we use Docker)

## The simple rule

| On your computer | Inside Docker |
|------------------|---------------|
| Git | Rust compiler |
| Docker | Tests and builds |
| Web browser | Postgres, Redis, MinIO, API, dashboard |

You should almost always type **`make …`**, not raw `cargo …`.

## Why

- Everyone uses the **same** tools and versions  
- CI (GitHub Actions) uses the **same** commands  
- No “works on my machine” surprises  

## Main folders

| Path | Role |
|------|------|
| `docker-compose.yml` | Starts all services |
| `docker/Dockerfile` | Production API image |
| `docker/Dockerfile.dev` | Build/test image |
| `docker/Dockerfile.web` | Dashboard image |
| `Makefile` | Easy commands that call Docker |
| `scripts/` | Helpers (`cargo.sh`, hooks) |

## Common flows

```bash
make bootstrap   # first setup
make up          # run the product
make test        # run tests
make down        # stop
```

## Clean slate

```bash
make down
make clean       # also removes Docker volumes/caches — next build is slower
```
