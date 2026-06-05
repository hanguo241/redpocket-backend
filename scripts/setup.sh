#!/usr/bin/env bash
set -euo pipefail

echo "=== RedPacket Backend Setup ==="

# 1. Check prerequisites
echo ">>> Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { echo "cargo is required. Install Rust: https://rustup.rs"; exit 1; }
command -v psql >/dev/null 2>&1 || { echo "psql is required. Install PostgreSQL."; exit 1; }

# 2. Load env vars
if [ -f .env ]; then
    set -a
    source .env
    set +a
elif [ -f backend/.env ]; then
    set -a
    source backend/.env
    set +a
fi

DB_URL="${DATABASE_URL:-postgres://pony@localhost:5432/redpacket}"

# 3. Create database if not exists
echo ">>> Creating database..."
DB_NAME=$(echo "$DB_URL" | sed 's/.*\/\([^?]*\)/\1/')
psql -d postgres -tc "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 \
    && echo "Database '$DB_NAME' already exists" \
    || { createdb "$DB_NAME" && echo "Created database '$DB_NAME'"; }

# 4. Run cargo check
echo ">>> Checking compilation..."
cargo check --manifest-path Cargo.toml

echo ""
echo "=== Setup complete! ==="
echo "Run 'cargo run' to start the server."
