#!/usr/bin/env bash
# Run arbitrary cargo commands inside the dev container.
# Usage: ./scripts/cargo.sh test -p drp-core
set -euo pipefail
cd "$(dirname "$0")/.."

if command -v docker-compose >/dev/null 2>&1; then
  DC=docker-compose
elif docker compose version >/dev/null 2>&1; then
  DC="docker compose"
else
  echo "docker-compose not found" >&2
  exit 1
fi

exec $DC run --rm dev cargo "$@"
