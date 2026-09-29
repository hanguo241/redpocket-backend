#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/03-migrate-db.sh"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -eq 0 ]] || fail "Unexpected arguments."
init_step
require_commands psql python3
[[ -x $CARGO/sqlx ]] || fail "Missing SQLx CLI; run 01-install-env.sh sqlx."
[[ $("$CARGO/sqlx" --version) == 'sqlx-cli 0.8.6' ]] || fail "SQLx CLI 0.8.6 is required; run 01-install-env.sh sqlx."
if [[ -f /etc/redpacket/initializing ]]; then
  python3 -c 'import bcrypt' || fail "Missing python3-bcrypt."
fi
load_config
# Reject legacy databases lacking migration tracking rather than replaying seed updates.
legacy=$(pg -d redpacket -Atc "SELECT to_regclass('public.packets') IS NOT NULL AND to_regclass('public._sqlx_migrations') IS NULL")
[[ $legacy == f ]] || fail "Legacy database has no SQLx history. Baseline it before deploying."
echo 'Applying tracked SQLx migrations'
"$CARGO/sqlx" migrate run --source "$ROOT/migrations"
if [[ -f /etc/redpacket/initializing ]]; then
  read -r -s -p 'New admin@redpacket.com password (12-72 bytes): ' ADMIN_PASSWORD </dev/tty; echo
  ADMIN_HASH=$(printf '%s' "$ADMIN_PASSWORD" | python3 -c 'import sys,bcrypt; p=sys.stdin.buffer.read(); assert 12 <= len(p) <= 72, "Password must contain 12-72 bytes"; print(bcrypt.hashpw(p,bcrypt.gensalt()).decode())')
  unset ADMIN_PASSWORD
  printf "UPDATE admin_users SET password_hash = '%s', updated_at = NOW() WHERE email = 'admin@redpacket.com';\n" "$ADMIN_HASH" | pg -d redpacket >/dev/null
  unset ADMIN_HASH
  unlink /etc/redpacket/initializing
fi
echo 'Migrations complete.'
