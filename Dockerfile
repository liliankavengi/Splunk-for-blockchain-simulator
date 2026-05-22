# ── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:1.79-slim AS builder

WORKDIR /build

# Cache dependency compilation separately from source
COPY Cargo.toml Cargo.lock ./
COPY simulator/Cargo.toml simulator/Cargo.toml
COPY assertions/Cargo.toml assertions/Cargo.toml

# Create stub lib/main files so `cargo fetch` can resolve the workspace
RUN mkdir -p simulator/src assertions/src && \
    echo 'fn main() {}' > simulator/src/main.rs && \
    echo 'fn main() {}' > assertions/src/main.rs && \
    cargo fetch

# Copy real source and build (no kafka feature — no C toolchain required)
COPY simulator/src simulator/src
COPY assertions/src assertions/src
COPY scenarios    scenarios

RUN cargo build --release -p simulator

# ── Stage 2: runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/simulator ./simulator
COPY --from=builder /build/scenarios               ./scenarios

# Render injects $PORT; the simulator reads it via the --dashboard-port / PORT env var.
ENV DASHBOARD_PASSWORD=changeme
ENV SIM_OUTPUT_FILE=-

EXPOSE 8080

# Default: run the flash-loan scenario, stream NDJSON to stdout.
# Override SIM_SCENARIO to pick a different scenario file at deploy time.
ENV SIM_SCENARIO=/app/scenarios/flash_loan_attack.json

CMD ["/bin/sh", "-c", "/app/simulator --scenario $SIM_SCENARIO"]
