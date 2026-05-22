# ── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:slim AS builder

# pkg-config + libssl-dev are needed by the clickhouse crate in the assertions
# workspace member (even though we only build the simulator binary, the
# workspace resolver downloads all crates).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY . .

# BuildKit cache mounts keep the Cargo registry and incremental build artefacts
# between Render builds, so only changed crates are recompiled.
# The binary is copied to /usr/local/bin before the cache mount is released.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p simulator && \
    cp target/release/simulator /usr/local/bin/simulator

# ── Stage 2: runtime ────────────────────────────────────────────────────────
# distroless/cc-debian12:nonroot — no shell, no package manager,
# runs as uid 65532, minimal CVE surface.
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /usr/local/bin/simulator ./simulator
COPY --from=builder /build/scenarios         ./scenarios

# Render injects $PORT; simulator reads it via the PORT env var.
# SIM_SCENARIO is read directly by clap — no shell required.
ENV DASHBOARD_PASSWORD=changeme
ENV SIM_OUTPUT_FILE=-
ENV SIM_SCENARIO=/app/scenarios/flash_loan_attack.json

EXPOSE 8080

ENTRYPOINT ["/app/simulator"]
