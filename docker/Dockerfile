# syntax=docker/dockerfile:1.6
# Multi-stage production image for the DRP API.
# Built only via Docker — no host toolchain required.

# ---------- builder ----------
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Workspace manifests + all members (crates *and* plugins/*)
COPY Cargo.toml rust-toolchain.toml rustfmt.toml clippy.toml ./
COPY Cargo.lock ./
COPY crates ./crates
COPY plugins ./plugins

RUN cargo build --release -p drp-api \
    && strip target/release/drp

# ---------- runtime ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 drp

WORKDIR /app

COPY --from=builder /app/target/release/drp /usr/local/bin/drp
COPY config ./config

ENV DRP_API_HOST=0.0.0.0 \
    DRP_API_PORT=8080 \
    DRP_ENV=production \
    RUST_LOG=info

EXPOSE 8080
USER drp

# Prefer readiness over bare liveness for orchestrators
HEALTHCHECK --interval=10s --timeout=3s --start-period=20s --retries=5 \
  CMD curl -fsS http://127.0.0.1:8080/readyz || exit 1

CMD ["drp"]
