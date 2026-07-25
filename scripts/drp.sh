#!/usr/bin/env bash
# Unified helper: ./scripts/drp.sh <make-target|cargo-args...>
# Examples:
#   ./scripts/drp.sh test
#   ./scripts/drp.sh cargo test -p drp-core
#   ./scripts/drp.sh up
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if docker compose version >/dev/null 2>&1; then
  DC="docker compose"
elif command -v docker-compose >/dev/null 2>&1; then
  DC=docker-compose
else
  echo "Docker Compose not found (need 'docker compose' or docker-compose)" >&2
  exit 1
fi

if [[ $# -eq 0 ]]; then
  exec make help
fi

cmd="$1"
shift || true

case "$cmd" in
  cargo)
    exec $DC run --rm --no-deps dev cargo "$@"
    ;;
  help|--help|-h)
    exec make help
    ;;
  *)
    # Delegate known/unknown make targets
    exec make "$cmd" "$@"
    ;;
esac
