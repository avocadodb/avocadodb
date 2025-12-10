# Multi-stage Dockerfile for AvocadoDB
# Optimized for fast multi-arch builds using cargo-zigbuild
# Builds both amd64 and arm64 via cross-compilation (no QEMU emulation)

# ===== Builder Stage =====
FROM rust:1.83-bookworm AS builder

# Install build dependencies and cross-compilation tools
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Use nightly toolchain first (required for cargo-zigbuild which uses edition 2024)
RUN rustup toolchain install nightly --profile minimal && rustup default nightly

# Install Zig (for cargo-zigbuild cross-compilation)
ARG ZIG_VERSION=0.13.0
RUN curl -L "https://ziglang.org/download/${ZIG_VERSION}/zig-linux-x86_64-${ZIG_VERSION}.tar.xz" | tar -xJ -C /usr/local \
    && ln -s /usr/local/zig-linux-x86_64-${ZIG_VERSION}/zig /usr/local/bin/zig

# Install cargo-zigbuild (requires nightly for edition 2024 deps)
RUN cargo install cargo-zigbuild

# Add Rust targets for cross-compilation
RUN rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

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

# Build dependencies for both architectures (cached layer)
RUN cargo zigbuild --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server --target x86_64-unknown-linux-gnu && \
    cargo zigbuild --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server --target aarch64-unknown-linux-gnu

# Remove dummy files and build artifacts for our crates only
RUN rm -rf target/x86_64-unknown-linux-gnu/release/.fingerprint/avocado-* \
    target/x86_64-unknown-linux-gnu/release/deps/avocado_* \
    target/aarch64-unknown-linux-gnu/release/.fingerprint/avocado-* \
    target/aarch64-unknown-linux-gnu/release/deps/avocado_* \
    avocado-*/src

# Copy actual source code
COPY avocado-core ./avocado-core
COPY avocado-server ./avocado-server

# Build the actual binaries for both architectures
RUN cargo zigbuild --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server --target x86_64-unknown-linux-gnu && \
    cargo zigbuild --release --locked --manifest-path avocado-server/Cargo.toml --bin avocado-server --target aarch64-unknown-linux-gnu

# Strip binaries to reduce size
RUN strip /build/target/x86_64-unknown-linux-gnu/release/avocado-server && \
    strip /build/target/aarch64-unknown-linux-gnu/release/avocado-server || true

# Organize binaries by platform for the runtime stage
RUN mkdir -p /out/linux/amd64 /out/linux/arm64 && \
    cp /build/target/x86_64-unknown-linux-gnu/release/avocado-server /out/linux/amd64/ && \
    cp /build/target/aarch64-unknown-linux-gnu/release/avocado-server /out/linux/arm64/

# ===== Runtime Stage =====
FROM debian:bookworm-slim

# Target platform is set by Docker Buildx
ARG TARGETPLATFORM

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

# Copy the appropriate binary based on target platform
COPY --from=builder /out/${TARGETPLATFORM}/avocado-server /usr/local/bin/avocado-server

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
