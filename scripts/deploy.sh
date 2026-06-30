#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Deploy ==="

# Configuration - customize for your environment
REMOTE_HOST="${DEPLOY_HOST:-}"
REMOTE_USER="${DEPLOY_USER:-deploy}"
REMOTE_PATH="${DEPLOY_PATH:-/opt/redpacket-backend}"
BINARY_NAMES=("redpacket-backend" "redpacket-sync-worker")

# 1. Build release binaries
echo ">>> Building release binaries..."
cargo build --release --bins --manifest-path Cargo.toml

# 2. Check if remote host is configured
if [ -z "$REMOTE_HOST" ]; then
    echo ""
    echo "=== Local build complete ==="
    for bin in "${BINARY_NAMES[@]}"; do
        echo "Binary: target/release/$bin"
    done
    echo ""
    echo "To deploy remotely, set environment variables:"
    echo "  export DEPLOY_HOST=your-server.com"
    echo "  export DEPLOY_USER=deploy"
    echo "  export DEPLOY_PATH=/opt/redpacket-backend"
    echo ""
    echo "Or manually:"
    for bin in "${BINARY_NAMES[@]}"; do
        echo "  scp target/release/$bin \$REMOTE_USER@\$REMOTE_HOST:\$REMOTE_PATH/"
    done
    echo "  scp .env \$REMOTE_USER@\$REMOTE_HOST:\$REMOTE_PATH/"
    echo "  ssh \$REMOTE_USER@\$REMOTE_HOST 'cd \$REMOTE_PATH && sudo systemctl restart redpacket-backend'"
    echo "  ssh \$REMOTE_USER@\$REMOTE_HOST 'cd \$REMOTE_PATH && sudo systemctl restart redpacket-sync-worker'"
    exit 0
fi

# 3. Deploy to remote
echo ">>> Deploying to $REMOTE_HOST..."
for bin in "${BINARY_NAMES[@]}"; do
    scp "target/release/$bin" "$REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"
done
scp backend/.env "$REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"

# 4. Restart services
echo ">>> Restarting services..."
for bin in "${BINARY_NAMES[@]}"; do
    ssh "$REMOTE_USER@$REMOTE_HOST" "cd $REMOTE_PATH && sudo systemctl restart $bin" || echo "Warning: Failed to restart $bin (may need sudo)"
done

echo "=== Deploy complete! ==="
