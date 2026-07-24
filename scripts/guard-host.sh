#!/usr/bin/env bash
# Fail CI/local scripts if someone tries to use host toolchains for this repo.
set -euo pipefail

if [[ "${DRP_ALLOW_HOST_CARGO:-}" == "1" ]]; then
  exit 0
fi

if [[ -z "${DRP_IN_CONTAINER:-}" ]]; then
  # Heuristic: /.dockerenv or cgroup
  if [[ ! -f /.dockerenv ]] && ! grep -qaE 'docker|containerd|kubepods' /proc/1/cgroup 2>/dev/null; then
    echo "ERROR: This project is container-first." >&2
    echo "       Run via: make build | make test | make shell | ./scripts/cargo.sh ..." >&2
    echo "       Do not invoke cargo/rustc on the host." >&2
    exit 1
  fi
fi
