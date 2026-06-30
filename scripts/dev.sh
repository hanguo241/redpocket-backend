#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Dev Server ==="

# 1. Start sync-worker in background
echo ">>> Starting sync-worker (background)..."
cargo run --bin redpacket-sync-worker &
SYNC_PID=$!

# 2. Start main backend
echo ">>> Starting main backend..."
if command -v cargo-watch >/dev/null 2>&1; then
    echo ">>> (with hot reload via cargo-watch)"
    cargo watch -x "run --bin redpacket-backend"
else
    cargo run --bin redpacket-backend
fi

# Cleanup: stop sync-worker when main process exits
echo ">>> Stopping sync-worker (pid=$SYNC_PID)..."
kill $SYNC_PID 2>/dev/null || true
