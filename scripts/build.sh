#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Build ==="

BUILD_PROFILE="${1:-release}"

case "$BUILD_PROFILE" in
    release)
        echo ">>> Building release binary..."
        cargo build --release
        echo ">>> Binary: target/release/redpacket-backend"
        ;;
    dev|debug)
        echo ">>> Building debug binary..."
        cargo build
        echo ">>> Binary: target/debug/redpacket-backend"
        ;;
    *)
        echo "Usage: $0 [release|debug]"
        exit 1
        ;;
esac

echo ">>> Build complete!"
