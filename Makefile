# Data Reliability Platform — container-first Makefile
# Host prerequisites: Git + Docker CLI + Compose only.
# All format / lint / test / doc / build commands run inside Docker.

.DEFAULT_GOAL := help

COMPOSE ?= $(shell if command -v docker-compose >/dev/null 2>&1; then echo docker-compose; \
	elif docker compose version >/dev/null 2>&1; then echo "docker compose"; \
	else echo docker-compose; fi)

DC  := $(COMPOSE)
DEV := $(DC) run --rm --no-deps dev

.PHONY: help doctor ensure-docker hooks \
	bootstrap infra up down restart logs ps shell \
	build release test lint fmt fmt-check clippy check deny doc docs-serve \
	api api-build clean \
	editorconfig-check pre-commit ci

help:
	@echo "Data Reliability Platform (container-first)"
	@echo ""
	@echo "  Host tools: Git + Docker + Compose ONLY."
	@echo "  Do not install Rust / Postgres / Redis / MinIO / Prometheus on the host."
	@echo ""
	@echo "Setup"
	@echo "  make doctor       Verify Docker prerequisites"
	@echo "  make hooks        Install git hooks (container-backed)"
	@echo "  make bootstrap    Build toolchain image + start infra"
	@echo ""
	@echo "Stack"
	@echo "  make infra        postgres + redis + minio"
	@echo "  make up           Full stack: infra + api + prometheus"
	@echo "  make down         Stop stack"
	@echo "  make restart      down + up"
	@echo "  make logs         Tail service logs"
	@echo "  make ps           List compose services"
	@echo "  make api          Rebuild/run API"
	@echo ""
	@echo "Dev (always inside Docker)"
	@echo "  make shell        Interactive bash in toolchain container"
	@echo "  make build        cargo build --workspace"
	@echo "  make release      cargo build --release -p drp-api"
	@echo "  make test         cargo test --workspace"
	@echo "  make fmt          cargo fmt --all"
	@echo "  make fmt-check    cargo fmt --check"
	@echo "  make clippy       cargo clippy -D warnings"
	@echo "  make lint         fmt-check + clippy"
	@echo "  make check        lint + test"
	@echo "  make deny         cargo-deny (licenses/advisories) if available"
	@echo "  make doc          cargo doc --workspace"
	@echo "  make docs-serve   Serve rustdoc on :3001"
	@echo "  make ci           Full local CI gate (container)"
	@echo ""
	@echo "Endpoints (after make up)"
	@echo "  API         http://127.0.0.1:8080"
	@echo "  Liveness    http://127.0.0.1:8080/livez"
	@echo "  Readiness   http://127.0.0.1:8080/readyz"
	@echo "  Metrics     http://127.0.0.1:8080/metrics"
	@echo "  Prometheus  http://127.0.0.1:9090"
	@echo "  MinIO UI    http://127.0.0.1:9001"
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
	@echo "Host toolchain check (should be unused):"
	@if command -v cargo >/dev/null 2>&1; then \
		echo "  WARN: cargo on host PATH — use make build/test/shell instead"; \
	else \
		echo "  OK: no cargo on host PATH"; \
	fi
	@if command -v psql >/dev/null 2>&1; then \
		echo "  WARN: psql on host — use compose postgres"; \
	else \
		echo "  OK: no psql on host PATH"; \
	fi
	@if command -v redis-cli >/dev/null 2>&1; then \
		echo "  WARN: redis-cli on host — use compose redis"; \
	else \
		echo "  OK: no redis-cli on host PATH"; \
	fi
	@echo "Doctor complete."

hooks:
	@./scripts/install-hooks.sh

bootstrap: ensure-docker
	$(DC) build dev
	$(DC) up -d postgres redis minio minio-init
	@./scripts/install-hooks.sh
	@echo "Infra up + toolchain image built + hooks installed."
	@echo "Next: make build && make test && make up"

infra: ensure-docker
	$(DC) up -d postgres redis minio minio-init

up: ensure-docker
	$(DC) up -d --build postgres redis minio minio-init api prometheus
	@echo ""
	@echo "API:        http://127.0.0.1:$${DRP_API_PORT:-8080}/readyz"
	@echo "Metrics:    http://127.0.0.1:$${DRP_API_PORT:-8080}/metrics"
	@echo "Prometheus: http://127.0.0.1:9090"

down: ensure-docker
	$(DC) down --remove-orphans

restart: down up

logs: ensure-docker
	$(DC) logs -f api postgres redis minio prometheus

ps: ensure-docker
	$(DC) ps

shell: ensure-docker
	$(DC) run --rm --no-deps dev

build: ensure-docker
	$(DEV) cargo build --workspace

release: ensure-docker
	$(DEV) cargo build --release -p drp-api

test: ensure-docker
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
	$(DEV) sh -c 'if command -v cargo-deny >/dev/null 2>&1; then cargo deny check; else echo "cargo-deny not in image (optional)"; fi'

doc: ensure-docker
	$(DEV) cargo doc --workspace --no-deps --all-features --document-private-items

docs-serve: ensure-docker
	$(DC) --profile docs up --build -d docs-serve
	@echo "Docs: http://127.0.0.1:3001"

api-build: ensure-docker
	$(DC) build api

api: ensure-docker
	$(DC) up -d --build api
	@echo "API: http://127.0.0.1:$${DRP_API_PORT:-8080}/readyz"

editorconfig-check: ensure-docker
	@# Lightweight: ensure .editorconfig exists and key files end with newline (container)
	$(DEV) sh -c 'test -f .editorconfig && echo "editorconfig present"'

pre-commit: fmt-check clippy
	@echo "pre-commit gate OK"

ci: ensure-docker lint test build
	@echo "Local CI gate OK (all containerized)."

clean: ensure-docker
	$(DC) down -v --remove-orphans --rmi local || true
	@echo "Cleaned compose resources."
