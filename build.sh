#!/bin/bash
set -e

PROJECT_DIR="$(pwd)"
IMAGE="rust:1.88-slim"

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     OS_NAME="linux";;
    Darwin*)    OS_NAME="macos";;
    *)          echo "Unsupported OS: ${OS}"; exit 1;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64*)    RUST_ARCH="x86_64";;
    amd64*)     RUST_ARCH="x86_64";;
    arm64*)     RUST_ARCH="aarch64";;
    aarch64*)   RUST_ARCH="aarch64";;
    *)          echo "Unsupported architecture: ${ARCH}"; exit 1;;
esac

# Set target triple
if [ "${OS_NAME}" = "linux" ]; then
    TARGET="${RUST_ARCH}-unknown-linux-gnu"
elif [ "${OS_NAME}" = "macos" ]; then
    TARGET="${RUST_ARCH}-apple-darwin"
fi

echo "Building for ${OS_NAME} on ${ARCH} (${TARGET})"

# Run build in Docker
docker run --rm \
    -v "${PROJECT_DIR}":/workspace \
    -w /workspace \
    ${IMAGE} \
    bash -c "rustup target add ${TARGET} && \
             cargo clean && \
             cargo build --release --target ${TARGET}"

echo "Optimized binary at: target/${TARGET}/release/git-remote-codecommit"

# Set executable permissions
chmod +x "target/${TARGET}/release/git-remote-codecommit"