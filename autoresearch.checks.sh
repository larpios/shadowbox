#!/bin/bash
set -euo pipefail

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "cargo not found"
    exit 1
fi

# Check for build
if ! cargo build; then
    echo "build failed"
    exit 1
fi

# Check for tests
if ! cargo test; then
    echo "tests failed"
    exit 1
fi

echo "all checks passed"
