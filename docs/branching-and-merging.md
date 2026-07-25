# Branches and merging

## Policy in plain words

- **`main` is protected.** You cannot push commits directly to it.
- All changes go through a **feature branch** and a **Pull Request (PR)**.
- **GitHub Actions** must pass (tests in Docker).
- **Only the repository owner** merges into `main`.

## How to open a PR

```bash
git checkout main
git pull origin main
git checkout -b feature/my-feature

# make your changes
make lint && make test

git add -A
git commit -m "feat: clear short summary"
git push -u origin HEAD
gh pr create --base main
```

## Branch naming ideas

- `feature/...` — new capability  
- `fix/...` — bug fix  
- `docs/...` — documentation only  
- `chore/...` — tooling / cleanup  

## After your PR is open

1. Wait for the green CI check  
2. Address review comments if any  
3. Owner merges when ready  

## Do not

- Force-push to `main`  
- Merge your own PR if you are not the owner (policy)  
- Skip `make test` when you change code  
