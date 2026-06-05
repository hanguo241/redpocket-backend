#!/usr/bin/env bash
set -euo pipefail

echo "=== Run Database Migrations ==="

# Load env
if [ -f .env ]; then
    set -a; source .env; set +a
elif [ -f backend/.env ]; then
    set -a; source backend/.env; set +a
fi

DB_URL="${DATABASE_URL:-postgres://pony@localhost:5432/redpacket}"

# Use sqlx migrate
echo ">>> Running migrations on $DB_URL"
sqlx migrate run --database-url "$DB_URL"

echo ">>> Migrations complete!"
