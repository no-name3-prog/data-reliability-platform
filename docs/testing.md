# Testing

All automated tests run **inside Docker**.  
Do **not** run `cargo test` on your laptop for this project.

## Quick commands

| Command | What it does |
|---------|----------------|
| `make test` | Run the full test suite |
| `make test-unit` | Fast unit tests |
| `make test-integration` | Multi-part flow tests |
| `make test-regression` | “Golden” expected results |
| `make lint` | Style + safety checks |
| `make verify` | Lint + all tests + build (like CI) |

## Before you open a PR

```bash
make lint
make test
```

Or everything at once:

```bash
make verify
```

## Test layers (simple)

1. **Unit** — small pieces of code (one rule, one parser, …)
2. **Integration** — several parts working together (discover → profile → validate)
3. **Regression** — fixed sample data must keep the same quality outcomes

## Dummy data in tests

Tests use built-in **mock** data (orders/users), not your production databases.

## Manual testing with the UI

Automated tests do **not** need `make up`.

To click through the app yourself:

1. `make up`
2. Open http://127.0.0.1:3000
3. Follow [getting-started.md](getting-started.md)

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Docker not running | Start Docker, then `make doctor` |
| Tests very slow first time | Normal — images and caches download once |
| Host `cargo` fails | Expected — use `make test` |
