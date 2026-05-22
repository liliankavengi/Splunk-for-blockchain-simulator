# ── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:1.79-slim AS builder

WORKDIR /build

# Copy workspace manifests + lock file so the dep-cache layer is independent
# of source changes.
COPY Cargo.toml Cargo.lock ./
COPY simulator/Cargo.toml  simulator/Cargo.toml
COPY assertions/Cargo.toml assertions/Cargo.toml

# Build deps with stub binaries, then throw away the stub artifacts.
# Any subsequent source-only change reuses this cached layer.
RUN mkdir -p simulator/src assertions/src && \
    printf 'fn main(){}' > simulator/src/main.rs && \
    printf 'fn main(){}' > assertions/src/main.rs && \
    cargo build --release -p simulator && \
    rm -f  target/release/simulator \
           target/release/deps/simulator-* \
           target/release/.fingerprint/simulator-*/dep-* && \
    rm -rf simulator/src assertions/src

# Copy real source and rebuild (deps already compiled above)
COPY simulator/src simulator/src
COPY assertions/src assertions/src
COPY scenarios     scenarios

RUN cargo build --release -p simulator

# ── Stage 2: runtime ────────────────────────────────────────────────────────
# distroless/cc includes glibc + libgcc but no shell or package manager,
# drastically reducing the CVE surface vs debian:bookworm-slim.
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

COPY --from=builder /build/target/release/simulator ./simulator
COPY --from=builder /build/scenarios                ./scenarios

# Render injects $PORT; simulator reads it via the PORT env var.
# SIM_SCENARIO is read directly by clap (no shell needed).
ENV DASHBOARD_PASSWORD=changeme
ENV SIM_OUTPUT_FILE=-
ENV SIM_SCENARIO=/app/scenarios/flash_loan_attack.json

EXPOSE 8080

ENTRYPOINT ["/app/simulator"]
