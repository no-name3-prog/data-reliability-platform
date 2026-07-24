#!/usr/bin/env bash
# Install container-first git hooks (no host Rust, no Python required).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push 2>/dev/null || true

echo "Git hooks installed (core.hooksPath=.githooks)."
echo "  pre-commit → make fmt-check + make clippy  (Docker)"
echo "  pre-push   → make test                     (Docker)"
