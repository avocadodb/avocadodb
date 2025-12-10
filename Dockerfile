# Multi-stage Dockerfile for AvocadoDB
# Optimized for small image size and security

# ===== Builder Stage =====
FROM rust:1.83 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Use nightly toolchain to support crates using edition 2024 while keeping lockfile determinism
RUN rustup toolchain install nightly --profile minimal && rustup default nightly
# Create app directory
WORKDIR /build

# Copy workspace manifest and member manifests first (for layer caching)
COPY Cargo.toml Cargo.lock ./
COPY avocado-core/Cargo.toml ./avocado-core/
COPY avocado-server/Cargo.toml ./avocado-server/
COPY avocado-cli/Cargo.toml ./avocado-cli/
COPY tests/Cargo.toml ./tests/

# Create dummy source files to build dependencies
RUN mkdir -p avocado-core/src avocado-server/src tests/src && \
    echo "fn main() {}" > avocado-server/src/main.rs && \
    echo "pub fn dummy() {}" > avocado-core/src/lib.rs && \
    mkdir -p avocado-core/benches && \
    echo "fn main() {}" > avocado-core/benches/session_bench.rs && \
    echo "fn main() {}" > avocado-core/benches/warm_cold_bench.rs && \
    mkdir -p avocado-cli/benches && echo "fn main() {}" > avocado-cli/benches/embedding_bench.rs && \
    echo "pub fn dummy() {}" > tests/src/lib.rs

# Pin transitive deps to stable-compatible versions and build dependencies
RUN cargo build --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server

# Remove dummy files and build artifacts
RUN rm -rf target/release/.fingerprint/avocado-* \
    target/release/deps/avocado_* \
    target/release/avocado-server \
    avocado-*/src

# Copy actual source code
COPY avocado-core ./avocado-core
COPY avocado-server ./avocado-server

# Build the actual binary
RUN cargo build --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server && \
    strip /build/target/release/avocado-server

# ===== Runtime Stage =====
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash avocado && \
    mkdir -p /data && \
    chown -R avocado:avocado /data

# Copy binary from builder
COPY --from=builder /build/target/release/avocado-server /usr/local/bin/avocado-server

# Set ownership
RUN chown avocado:avocado /usr/local/bin/avocado-server

# Switch to non-root user
USER avocado
WORKDIR /data

# Expose default port
EXPOSE 8765

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/bin/sh", "-c", "command -v curl >/dev/null && curl -f http://localhost:8765/health || exit 1"]

# Set environment variables
ENV RUST_LOG=info \
    PORT=8765 \
    BIND_ADDR=0.0.0.0

# Run the server
CMD ["/usr/local/bin/avocado-server"]
