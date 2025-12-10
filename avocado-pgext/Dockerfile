# AvocadoDB PostgreSQL Extension Builder
# Builds the pgrx-based extension on pgvector/pgvector:pg16
# Multi-arch support with pre-downloaded ONNX Runtime

# Build stage
FROM rust:1.83-bookworm AS builder

# Detect target architecture
ARG TARGETARCH

# Install build dependencies
RUN apt-get update && apt-get install -y \
    postgresql-16 \
    postgresql-server-dev-16 \
    libclang-dev \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Pre-download ONNX Runtime to avoid TLS issues during build
# ort-sys v2.0.0-rc.10 uses ONNX Runtime 1.22.0
ARG ORT_VERSION=1.22.0
ENV ORT_LIB_LOCATION=/opt/onnxruntime

# Download ONNX Runtime based on target architecture
RUN mkdir -p /opt/onnxruntime && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        echo "Downloading ONNX Runtime for aarch64..." && \
        curl -sL "https://cdn.pyke.io/0/pyke:ort-rs/ms@${ORT_VERSION}/aarch64-unknown-linux-gnu.tgz" | tar -xz -C /opt/onnxruntime --strip-components=1; \
    else \
        echo "Downloading ONNX Runtime for x86_64..." && \
        curl -sL "https://cdn.pyke.io/0/pyke:ort-rs/ms@${ORT_VERSION}/x86_64-unknown-linux-gnu.tgz" | tar -xz -C /opt/onnxruntime --strip-components=1; \
    fi

# Install cargo-pgrx
RUN cargo install cargo-pgrx --version 0.12.9 --locked

# Initialize pgrx with PostgreSQL 16
RUN cargo pgrx init --pg16 /usr/lib/postgresql/16/bin/pg_config

# Set up build directory
WORKDIR /build

# Copy only what we need for the extension
COPY avocado-pgext ./avocado-pgext

# Build the extension
WORKDIR /build/avocado-pgext
RUN cargo pgrx package --pg-config /usr/lib/postgresql/16/bin/pg_config

# Runtime stage - based on pgvector
FROM pgvector/pgvector:pg16

# Copy extension files from builder
COPY --from=builder /build/avocado-pgext/target/release/avocado-pg16/usr/share/postgresql/16/extension/* \
    /usr/share/postgresql/16/extension/
COPY --from=builder /build/avocado-pgext/target/release/avocado-pg16/usr/lib/postgresql/16/lib/* \
    /usr/lib/postgresql/16/lib/

# Copy schema file
COPY --from=builder /build/avocado-pgext/sql/schema.sql /docker-entrypoint-initdb.d/01-schema.sql

# Create init script for extensions
RUN echo "CREATE EXTENSION IF NOT EXISTS vector;" > /docker-entrypoint-initdb.d/00-extensions.sql && \
    echo "CREATE EXTENSION IF NOT EXISTS avocado;" >> /docker-entrypoint-initdb.d/00-extensions.sql

# Environment variables for embedding configuration
# Default: fastembed (pure Rust, offline)
# Set AVOCADO_EMBEDDING_PROVIDER=ollama to use local Ollama
ENV AVOCADO_EMBEDDING_PROVIDER=fastembed
ENV AVOCADO_OLLAMA_URL=http://host.docker.internal:11434
ENV AVOCADO_OLLAMA_MODEL=bge-m3

# Labels
LABEL org.opencontainers.image.source="https://github.com/avocadodb/avocadodb"
LABEL org.opencontainers.image.description="AvocadoDB PostgreSQL Extension - Deterministic context compilation for AI agents"
LABEL org.opencontainers.image.licenses="MIT"

# Healthcheck
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD pg_isready -U postgres || exit 1

# Default PostgreSQL command
CMD ["postgres"]
