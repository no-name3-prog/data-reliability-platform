# Data Reliability Platform — container-first Makefile
# Host prerequisites: Git + Docker CLI + Compose only.
# All format / lint / test / doc / build commands run inside Docker.

.DEFAULT_GOAL := help

# Prefer Compose V2 (`docker compose`) — GitHub Actions and modern Docker Desktop.
# Fall back to standalone `docker-compose` only if the plugin is missing.
COMPOSE ?= $(shell if docker compose version >/dev/null 2>&1; then echo "docker compose"; \
	elif command -v docker-compose >/dev/null 2>&1; then echo docker-compose; \
	else echo "docker compose"; fi)

DC  := $(COMPOSE)
DEV := $(DC) run --rm --no-deps dev

.PHONY: help doctor ensure-docker hooks wait-infra bootstrap-image down-volumes smoke-up smoke-probe \
	bootstrap infra up down restart logs ps shell \
	build release test test-unit test-integration test-regression test-all test-cargo \
	lint fmt fmt-check clippy check deny doc docs-serve \
	api api-build clean \
	editorconfig-check pre-commit ci verify

help:
	@echo "Data Reliability Platform (container-first)"
	@echo ""
	@echo "  Host tools: Git + Docker + Compose ONLY."
	@echo ""
	@echo "Setup"
	@echo "  make doctor / hooks / bootstrap"
	@echo ""
	@echo "Stack"
	@echo "  make infra | up | down | logs | ps | api | web"
	@echo ""
	@echo "Quality (Docker)"
	@echo "  make build | release | lint | fmt | clippy | doc"
	@echo ""
	@echo "Testing (cargo-nextest inside Docker)"
	@echo "  make test                Full suite (nextest profile ci)"
	@echo "  make test-unit           Unit tests only"
	@echo "  make test-integration    Integration tests"
	@echo "  make test-regression     Regression / golden fixtures"
	@echo "  make test-all            unit + integration + regression"
	@echo "  make test-cargo          cargo test (fallback, no nextest)"
	@echo "  make verify              lint + test-all + build (local CI mirror)"
	@echo "  make ci                  Same as verify (CI entrypoint)"
	@echo ""
	@echo "Helpers: ./scripts/drp.sh <target>   ./scripts/cargo.sh <args>"

ensure-docker:
	@command -v docker >/dev/null 2>&1 || { echo "ERROR: docker CLI not found."; exit 1; }
	@command -v docker-compose >/dev/null 2>&1 || docker compose version >/dev/null 2>&1 || { \
		echo "ERROR: docker-compose not found."; exit 1; }
	@docker info >/dev/null 2>&1 || { echo "ERROR: Docker daemon not running."; exit 1; }

doctor: ensure-docker
	@echo "Compose driver: $(DC)"
	@docker version --format 'Docker client {{.Client.Version}} / server {{.Server.Version}}'
	@echo ""
	@if command -v cargo >/dev/null 2>&1; then \
		echo "  WARN: cargo on host PATH — do not use it; use make test / make shell"; \
	else \
		echo "  OK: no cargo on host PATH"; \
	fi
	@echo "Doctor complete."

hooks:
	@./scripts/install-hooks.sh

bootstrap-image: ensure-docker
	$(DC) build dev

bootstrap: ensure-docker bootstrap-image
	$(DC) up -d postgres redis minio minio-init
	@./scripts/install-hooks.sh || true
	@echo "Infra up + toolchain (incl. nextest) + hooks. Next: make verify"

infra: ensure-docker
	$(DC) up -d postgres redis minio minio-init

wait-infra: ensure-docker
	@echo "Waiting for postgres/redis healthy..."
	@for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do \
	  if $(DC) exec -T postgres pg_isready -U $${POSTGRES_USER:-drp} -d $${POSTGRES_DB:-drp} >/dev/null 2>&1 \
	    && $(DC) exec -T redis redis-cli ping 2>/dev/null | grep -q PONG; then \
	      echo "Infrastructure is healthy."; exit 0; \
	  fi; \
	  sleep 2; \
	done; \
	echo "ERROR: infrastructure not healthy in time"; $(DC) ps; exit 1

up: ensure-docker
	$(DC) up -d --build postgres redis minio minio-init api prometheus web
	@echo "Dashboard http://127.0.0.1:$${DRP_WEB_PORT:-3000}"
	@echo "API       http://127.0.0.1:$${DRP_API_PORT:-8080}/readyz  Prometheus :9090"

down: ensure-docker
	$(DC) down --remove-orphans

down-volumes: ensure-docker
	$(DC) down -v --remove-orphans

restart: down up

logs: ensure-docker
	$(DC) logs -f api postgres redis minio prometheus

ps: ensure-docker
	$(DC) ps

shell: ensure-docker
	$(DC) run --rm --no-deps dev

build: ensure-docker
	$(DEV) cargo build --workspace --all-targets

release: ensure-docker
	$(DEV) cargo build --release -p drp-api

# --- Testing (always containerized) ---

test: ensure-docker
	$(DEV) cargo nextest run --workspace --all-features --profile ci

test-unit: ensure-docker
	$(DEV) cargo nextest run --workspace --all-features --profile unit

test-integration: ensure-docker
	$(DEV) cargo nextest run --workspace --all-features --profile integration

test-regression: ensure-docker
	$(DEV) cargo nextest run --workspace --all-features --profile regression

test-all: test-unit test-integration test-regression
	@echo "All test profiles OK (container)."

test-cargo: ensure-docker
	$(DEV) cargo test --workspace --all-features

fmt: ensure-docker
	$(DEV) cargo fmt --all

fmt-check: ensure-docker
	$(DEV) cargo fmt --all -- --check

clippy: ensure-docker
	$(DEV) cargo clippy --workspace --all-targets --all-features -- -D warnings

lint: fmt-check clippy
	@echo "Lint OK (container)."

check: lint test
	@echo "Check OK (container)."

deny: ensure-docker
	$(DEV) sh -c 'if command -v cargo-deny >/dev/null 2>&1; then cargo deny check; else echo "cargo-deny optional"; fi'

doc: ensure-docker
	$(DEV) cargo doc --workspace --no-deps --all-features --document-private-items

docs-serve: ensure-docker
	$(DC) --profile docs up --build -d docs-serve
	@echo "Docs: http://127.0.0.1:3001"

api-build: ensure-docker
	$(DC) build api

api: ensure-docker
	$(DC) up -d --build api

editorconfig-check: ensure-docker
	$(DEV) sh -c 'test -f .editorconfig && test -f .config/nextest.toml && echo "tooling config present"'

pre-commit: fmt-check clippy
	@echo "pre-commit gate OK"

# Mirror of GitHub Actions quality job
verify: ensure-docker lint test-all build doc
	@echo "=========================================="
	@echo " VERIFY OK — matches CI quality pipeline"
	@echo " (all steps ran inside Docker containers)"
	@echo "=========================================="

ci: verify

web-build: ensure-docker
	$(DC) build web

web: ensure-docker
	$(DC) up -d web
	@echo "Dashboard: http://127.0.0.1:$${DRP_WEB_PORT:-3000}"


clean: ensure-docker
	$(DC) down -v --remove-orphans --rmi local || true
	@echo "Cleaned compose resources."

# CI smoke helpers (Compose V2 via $(DC))
smoke-up: ensure-docker
	$(DC) up -d --build postgres redis minio minio-init api

smoke-probe: ensure-docker
	@set -e; \
	for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40; do \
	  if curl -fsS http://127.0.0.1:$${DRP_API_PORT:-8080}/readyz; then \
	    echo; curl -fsS http://127.0.0.1:$${DRP_API_PORT:-8080}/livez; echo; \
	    curl -fsS http://127.0.0.1:$${DRP_API_PORT:-8080}/metrics | head -30; echo; \
	    curl -fsS http://127.0.0.1:$${DRP_API_PORT:-8080}/v1/plugins | head -c 200; echo; \
	    exit 0; \
	  fi; \
	  sleep 3; \
	done; \
	echo "ERROR: API not ready"; $(DC) logs api || true; exit 1
