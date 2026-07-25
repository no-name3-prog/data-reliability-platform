# Contributing

Thanks for helping improve the Data Reliability Platform.

## Rules (please read)

1. **Do not push to `main`.** GitHub blocks it. Always use a **branch + Pull Request**.
2. **Use Docker.** Do not install Rust or databases just for this project.
3. **Run tests before you open a PR:** `make lint` and `make test`.
4. **Only the repo owner merges** PRs into `main`.

## What you need

- Git  
- Docker (running)

## First-time setup

```bash
git clone https://github.com/no-name3-prog/data-reliability-platform.git
cd data-reliability-platform
make doctor
make bootstrap
make test
```

## Make a change (step by step)

```bash
# 1. Start from latest main
git checkout main
git pull origin main

# 2. Create your branch (pick a clear name)
git checkout -b feature/short-description

# 3. Edit the code or docs

# 4. Check your work (runs inside Docker)
make lint
make test

# 5. Commit and push
git add -A
git commit -m "feat: describe your change in one short line"
git push -u origin HEAD

# 6. Open a Pull Request against main
gh pr create --base main
# or open the link GitHub prints after push
```

### Good commit messages

- `feat: add email validation rule`
- `fix: handle empty profile history`
- `docs: simplify getting started guide`

## Useful commands

| Task | Command |
|------|---------|
| Format code | `make fmt` |
| Lint | `make lint` |
| Tests | `make test` |
| Full check (like CI) | `make verify` |
| Start app + UI | `make up` |
| Stop app | `make down` |

## Pull Request checklist

- [ ] Branch is based on latest `main`
- [ ] `make lint` passes
- [ ] `make test` passes
- [ ] PR description explains **what** and **why**
- [ ] Docs updated if you changed behavior users see

## Git hooks

After `make bootstrap`, Git will run checks before commit/push (in Docker).  
If a hook fails, fix the error and try again.

## Where to put code

| If you are adding… | Look at |
|--------------------|---------|
| A new connector / rule / notifier | [docs/contributing-plugins.md](docs/contributing-plugins.md) |
| API routes | `crates/drp-api` |
| Dashboard UI | `web/` |
| Docs | `docs/` or `README.md` |

## Need help?

- [Getting started](docs/getting-started.md)  
- [Testing](docs/testing.md)  
- [Branching policy](docs/branching-and-merging.md)  

Thank you!
