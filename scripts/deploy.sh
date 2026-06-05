#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Deploy ==="

# Configuration - customize for your environment
REMOTE_HOST="${DEPLOY_HOST:-}"
REMOTE_USER="${DEPLOY_USER:-deploy}"
REMOTE_PATH="${DEPLOY_PATH:-/opt/redpacket-backend}"
BINARY_NAME="redpacket-backend"

# 1. Build release binary
echo ">>> Building release binary..."
cargo build --release --manifest-path Cargo.toml

# 2. Check if remote host is configured
if [ -z "$REMOTE_HOST" ]; then
    echo ""
    echo "=== Local build complete ==="
    echo "Binary: target/release/$BINARY_NAME"
    echo ""
    echo "To deploy remotely, set environment variables:"
    echo "  export DEPLOY_HOST=your-server.com"
    echo "  export DEPLOY_USER=deploy"
    echo "  export DEPLOY_PATH=/opt/redpacket-backend"
    echo ""
    echo "Or manually:"
    echo "  scp target/release/$BINARY_NAME $REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"
    echo "  scp .env $REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"
    echo "  ssh $REMOTE_USER@$REMOTE_HOST 'cd $REMOTE_PATH && systemctl restart redpacket-backend'"
    exit 0
fi

# 3. Deploy to remote
echo ">>> Deploying to $REMOTE_HOST..."
scp "target/release/$BINARY_NAME" "$REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"
scp backend/.env "$REMOTE_USER@$REMOTE_HOST:$REMOTE_PATH/"

# 4. Restart service
echo ">>> Restarting service..."
ssh "$REMOTE_USER@$REMOTE_HOST" "cd $REMOTE_PATH && systemctl restart $BINARY_NAME"

echo "=== Deploy complete! ==="
