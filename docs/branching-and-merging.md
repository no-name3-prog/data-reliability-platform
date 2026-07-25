# Branching and merging policy

## Rules (enforced by GitHub)

| Rule | Enforcement |
|------|-------------|
| No direct commits / pushes to `main` | Branch protection + `enforce_admins` |
| Changes land via pull request only | Required pull request |
| CI must pass before merge | Required check: `Lint · Unit · Integration · Regression · Build · Docs` |
| Branch up to date with `main` | Strict status checks |
| Conversations resolved | Required conversation resolution |
| No force-push / delete of `main` | Branch protection |

## Who may merge

- **Only the repository owner** (`@no-name3-prog`) should merge pull requests.
- Personal GitHub accounts cannot use “restrict push to listed users” (org-only). Operationally:
  - Do **not** grant collaborators **Write**/**Maintain** unless you trust them to follow this policy.
  - Prefer **fork + PR**, or invite collaborators with **Triage**/**Read** only.
  - Owner merges after review and green CI.

## Contributor workflow

```bash
# 1. Sync
git fetch origin
git checkout main
git pull origin main

# 2. Feature branch (required)
git checkout -b feature/short-description

# 3. Work (always containerized)
make lint
make test
# …

# 4. Push branch (not main)
git push -u origin feature/short-description

# 5. Open PR targeting main
gh pr create --base main --title "…" --body "…"

# 6. Wait for CI (Docker) — owner merges when ready
```

Do **not**:

```bash
git push origin main          # blocked
git commit && push on main    # blocked
```

## Owner merge checklist

1. CI quality job green  
2. Review code / docs  
3. Resolve conversations  
4. Merge (squash or merge commit per preference)  
5. Delete feature branch (optional)

```bash
gh pr merge <number> --squash --delete-branch
```
