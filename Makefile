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
	@echo "  make infra | up | down | logs | ps | api"
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

bootstrap: ensure-docker
	$(DC) build dev
	$(DC) up -d postgres redis minio minio-init
	@./scripts/install-hooks.sh
	@echo "Infra up + toolchain (incl. nextest) + hooks. Next: make verify"

infra: ensure-docker
	$(DC) up -d postgres redis minio minio-init

up: ensure-docker
	$(DC) up -d --build postgres redis minio minio-init api prometheus
	@echo "API http://127.0.0.1:$${DRP_API_PORT:-8080}/readyz  Prometheus :9090"

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

clean: ensure-docker
	$(DC) down -v --remove-orphans --rmi local || true
	@echo "Cleaned compose resources."
