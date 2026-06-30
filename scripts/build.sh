#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Build ==="

BUILD_PROFILE="${1:-release}"

case "$BUILD_PROFILE" in
    release)
        echo ">>> Building release binaries..."
        cargo build --release --bins
        echo ">>> Binaries:"
        echo "  - target/release/redpacket-backend"
        echo "  - target/release/redpacket-sync-worker"
        ;;
    dev|debug)
        echo ">>> Building debug binaries..."
        cargo build --bins
        echo ">>> Binaries:"
        echo "  - target/debug/redpacket-backend"
        echo "  - target/debug/redpacket-sync-worker"
        ;;
    *)
        echo "Usage: $0 [release|debug]"
        exit 1
        ;;
esac

echo ">>> Build complete!"
