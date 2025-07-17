#!/bin/bash
set -e

PROJECT_DIR="$(pwd)"
IMAGE="rust:1.88-slim"

docker run --rm \
    -v "$PROJECT_DIR":/workspace \
    -w /workspace \
    $IMAGE \
    bash -c "cargo clean && \
             cargo build --release"

echo "Optimized binary at: target/release/git-remote-codecommit"