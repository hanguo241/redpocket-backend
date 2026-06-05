#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Dev Server ==="

# Try cargo-watch for hot reload, fallback to cargo run
if command -v cargo-watch >/dev/null 2>&1; then
    echo ">>> Starting with hot reload (cargo-watch)..."
    cargo watch -x run
else
    echo ">>> cargo-watch not found. Install with: cargo install cargo-watch"
    echo ">>> Starting without hot reload..."
    cargo run
fi
