#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/02-init-db.sh"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -eq 0 ]] || fail "Unexpected arguments."
init_step
require_commands psql openssl python3
python3 -c 'import bcrypt' || fail "Missing python3-bcrypt."
pg -d postgres -Atc 'SELECT 1' >/dev/null
install -d -m 700 /etc/redpacket
if [[ ! -f $CONFIG ]]; then
  ROLE_EXISTS=$(pg -d postgres -Atc "SELECT count(*) FROM pg_roles WHERE rolname='redpacket'")
  DB_EXISTS=$(pg -d postgres -Atc "SELECT count(*) FROM pg_database WHERE datname='redpacket'")
  if [[ $ROLE_EXISTS == 1 ]]; then
    read -r -s -p 'Existing redpacket database password (will not be changed): ' DB_PASSWORD </dev/tty; echo
    [[ -n $DB_PASSWORD ]] || fail "Existing password is required; no credentials were changed."
    CHECK_DATABASE=postgres
    [[ $DB_EXISTS == 0 ]] || CHECK_DATABASE=redpacket
    PGPASSWORD="$DB_PASSWORD" psql -X -w -h 127.0.0.1 -p 5432 -U redpacket -d "$CHECK_DATABASE" -v ON_ERROR_STOP=1 -Atc 'SELECT 1' >/dev/null || fail "Cannot authenticate with existing credentials; no configuration was written."
  else
    [[ $DB_EXISTS == 0 ]] || fail "Database exists without the redpacket role; configure its existing owner deliberately."
    DB_PASSWORD=$(openssl rand -hex 32)
  fi
  DB_PASSWORD_ENCODED=$(printf '%s' "$DB_PASSWORD" | python3 -c 'import sys,urllib.parse; print(urllib.parse.quote(sys.stdin.read(), safe=""))')
  unset DB_PASSWORD
  read -r -s -p 'SIGNER_PRIVATE_KEY (existing platform key, hex): ' SIGNER </dev/tty; echo
  read -r -s -p 'RELAYER_PRIVATE_KEY (separate funded wallet, hex): ' RELAYER </dev/tty; echo
  for key in "$SIGNER" "$RELAYER"; do
    [[ $key =~ ^(0x)?[0-9a-fA-F]{64}$ && ! $key =~ ^(0x)?0{64}$ ]] || fail "Invalid private key."
  done
  read -r -p 'Public website URL (https://...): ' SHARE </dev/tty
  [[ $SHARE =~ ^https://[a-zA-Z0-9.-]+(:[0-9]+)?(/)?$ ]] || fail "Supply an HTTPS website origin."
  JWT=$(openssl rand -hex 32)
  # Generated values have no shell/systemd quoting characters.
  cat > "$CONFIG" <<EOF
DATABASE_URL=postgres://redpacket:$DB_PASSWORD_ENCODED@127.0.0.1:5432/redpacket
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
SIGNER_PRIVATE_KEY=$SIGNER
RELAYER_PRIVATE_KEY=$RELAYER
JWT_SECRET=$JWT
SHARE_URL_HOST=$SHARE
RUST_LOG=info
EOF
  # Only a new database needs initial admin-password setup. Never reset existing admins.
  if [[ $DB_EXISTS == 0 ]]; then
    touch /etc/redpacket/initializing
  fi
  unset SIGNER RELAYER key JWT DB_PASSWORD_ENCODED
fi
load_config
if [[ -f /etc/redpacket/initializing ]]; then
  DB_PASSWORD=${DATABASE_URL#postgres://redpacket:}
  DB_PASSWORD=${DB_PASSWORD%@127.0.0.1:5432/redpacket}
  if [[ $(pg -d postgres -Atc "SELECT count(*) FROM pg_roles WHERE rolname='redpacket'") == 0 ]]; then
    [[ $DB_PASSWORD =~ ^[0-9a-f]{64}$ ]] || fail "Cannot recreate a missing role from reused credentials; restore the role deliberately."
    # Password goes over stdin, never through command arguments.
    printf "CREATE ROLE redpacket LOGIN PASSWORD '%s';\n" "$DB_PASSWORD" | pg -d postgres >/dev/null
  fi
  if [[ $(pg -d postgres -Atc "SELECT count(*) FROM pg_database WHERE datname='redpacket'") == 0 ]]; then
    pg -d postgres -c 'CREATE DATABASE redpacket OWNER redpacket' >/dev/null
  fi
  unset DB_PASSWORD
fi
echo 'Database/configuration ready. Next: 03-migrate-db.sh (new databases only: sets initial admin password).'
