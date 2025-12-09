#!/bin/bash
# Build script for AvocadoDB
# This script handles conda environment conflicts and OpenSSL version issues

set -e

# Unset conda compiler settings that interfere with Rust builds
unset CC CXX CFLAGS CXXFLAGS LDFLAGS
unset CONDA_PREFIX CONDA_DEFAULT_ENV CONDA_TOOLCHAIN_BUILD CONDA_TOOLCHAIN_HOST
unset CC_FOR_BUILD CXX_FOR_BUILD CONDA_BUILD_SYSROOT

# Ensure cargo is in PATH
if command -v cargo &> /dev/null; then
    echo "Using cargo from: $(which cargo)"
else
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        echo "Error: cargo not found. Please install Rust: https://rustup.rs"
        exit 1
    fi
fi

# Set clean PATH without conda
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"

# Build command
CMD="${1:-build}"
shift || true

echo "Running: cargo $CMD --workspace $*"
cargo "$CMD" --workspace "$@"
