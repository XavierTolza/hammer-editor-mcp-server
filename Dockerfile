# ── Build Stage ──────────────────────────────────────────────────
FROM rust:1.89-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY src/ ./src/

RUN cargo build --release

# ── Runtime Stage ───────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/hammer-editor-mcp-server /usr/local/bin/hammer-editor-mcp-server

ENV HAMMER_SERVER_URL=http://localhost:8080
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/hammer-editor-mcp-server"]