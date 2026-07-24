# Data Reliability Platform — container-first Makefile
# Host prerequisites: Git + Docker CLI + Compose. Nothing else.
#
# Never run cargo/rustc/psql/redis-cli against the host.
# All targets shell into containers.

.DEFAULT_GOAL := help

COMPOSE ?= $(shell if command -v docker-compose >/dev/null 2>&1; then echo docker-compose; \
	elif docker compose version >/dev/null 2>&1; then echo "docker compose"; \
	else echo docker-compose; fi)

DC := $(COMPOSE)
# --no-deps: unit build/test do not need postgres/redis/minio running
DEV := $(DC) run --rm --no-deps dev

.PHONY: help bootstrap up down logs ps shell \
	build test lint fmt check doc docs-serve \
	api api-build clean doctor ensure-docker infra

help:
	@echo "Data Reliability Platform (container-first)"
	@echo ""
	@echo "  Host tools required: Git + Docker + Compose only."
	@echo "  Do NOT install Rust, PostgreSQL, Redis, or MinIO on the host."
	@echo ""
	@echo "Infrastructure & runtime"
	@echo "  make doctor      Verify Docker prerequisites"
	@echo "  make bootstrap   Build toolchain image + start infra"
	@echo "  make infra       Start postgres/redis/minio only"
	@echo "  make up          Start full stack (infra + api)"
	@echo "  make down        Stop stack"
	@echo "  make logs        Tail logs"
	@echo "  make ps          Show services"
	@echo "  make api         Rebuild/run API image"
	@echo ""
	@echo "Dev toolchain (Docker only — never host cargo)"
	@echo "  make shell       bash in Rust toolchain container"
	@echo "  make build       cargo build --workspace"
	@echo "  make test        cargo test --workspace"
	@echo "  make lint        fmt check + clippy -D warnings"
	@echo "  make fmt         cargo fmt --all"
	@echo "  make check       lint + test"
	@echo "  make doc         cargo doc"
	@echo "  make docs-serve  rustdoc on :3001"
	@echo ""
	@echo "  make clean       Remove containers, volumes, caches"

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
		echo "  WARN: cargo on host PATH — do not use it; use make build/test/shell"; \
	else \
		echo "  OK: no cargo on host PATH"; \
	fi
	@if command -v psql >/dev/null 2>&1; then \
		echo "  WARN: psql on host — use compose postgres instead"; \
	else \
		echo "  OK: no psql on host PATH"; \
	fi
	@if command -v redis-cli >/dev/null 2>&1; then \
		echo "  WARN: redis-cli on host — use compose redis instead"; \
	else \
		echo "  OK: no redis-cli on host PATH"; \
	fi
	@echo "Doctor complete."

bootstrap: ensure-docker
	$(DC) build dev
	$(DC) up -d postgres redis minio minio-init
	@echo "Infra up + toolchain image built. Next: make build && make test"

infra: ensure-docker
	$(DC) up -d postgres redis minio minio-init

up: ensure-docker
	$(DC) up -d --build postgres redis minio minio-init api
	@echo "API: http://127.0.0.1:$${DRP_API_PORT:-8080}/health"

down: ensure-docker
	$(DC) down --remove-orphans

logs: ensure-docker
	$(DC) logs -f api postgres redis minio

ps: ensure-docker
	$(DC) ps

shell: ensure-docker
	$(DC) run --rm --no-deps dev

build: ensure-docker
	$(DEV) cargo build --workspace

test: ensure-docker
	$(DEV) cargo test --workspace --all-features

lint: ensure-docker
	$(DEV) sh -c 'cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings'

fmt: ensure-docker
	$(DEV) cargo fmt --all

check: lint test
	@echo "Container gate OK."

doc: ensure-docker
	$(DEV) cargo doc --workspace --no-deps --all-features --document-private-items

docs-serve: ensure-docker
	$(DC) --profile docs up --build -d docs-serve
	@echo "Docs: http://127.0.0.1:3001"

api-build: ensure-docker
	$(DC) build api

api: ensure-docker
	$(DC) up -d --build api
	@echo "API: http://127.0.0.1:$${DRP_API_PORT:-8080}/health"

clean: ensure-docker
	$(DC) down -v --remove-orphans --rmi local || true
	@echo "Cleaned compose resources."
