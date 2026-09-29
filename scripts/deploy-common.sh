#!/usr/bin/env bash
# Shared by numbered server deployment scripts; source only.
set -euo pipefail
set +x
umask 077
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
CONFIG=/etc/redpacket/backend.env
fail() { echo "ERROR: $*" >&2; exit 1; }
require_commands() {
  local command_name
  for command_name in "$@"; do
    command -v "$command_name" >/dev/null || fail "Missing $command_name; run 00-check-env.sh and install the required component."
  done
}
init_step() {
  [[ $EUID -eq 0 ]] || fail "Run with sudo from a regular login user."
  [[ -f /etc/os-release ]] || fail "Ubuntu 26.04 is required."
  # shellcheck disable=SC1091
  . /etc/os-release
  [[ $ID == ubuntu && $VERSION_ID == 26.04 ]] || fail "Only Ubuntu 26.04 is supported."
  [[ -n ${SUDO_USER:-} && $SUDO_USER != root ]] || fail "Use sudo from a regular login user."
  BUILD_USER=$SUDO_USER
  BUILD_HOME=$(getent passwd "$BUILD_USER" | cut -d: -f6)
  [[ -n $BUILD_HOME ]] || fail "Cannot find login user's home."
  CARGO="$BUILD_HOME/.cargo/bin"
  require_commands flock runuser
  exec 9>/run/lock/redpacket-deploy.lock
  flock -n 9 || fail "Another deployment step is running."
  trap 'echo "Step failed at line $LINENO; completed changes have not been rolled back." >&2' ERR
  cd "$ROOT"
}
as_build() { runuser -u "$BUILD_USER" -- env -i HOME="$BUILD_HOME" PATH="$CARGO:/usr/local/bin:/usr/bin:/bin" "$@"; }
pg() { runuser -u postgres -- psql -X -v ON_ERROR_STOP=1 "$@"; }
load_config() {
  [[ -f $CONFIG ]] || fail "Missing $CONFIG; run 02-init-db.sh or prepare your existing production configuration."
  [[ $(stat -c '%U:%a' "$CONFIG") == root:600 ]] || fail "Configuration must be owned by root with mode 600."
  set -a
  # shellcheck disable=SC1090
  . "$CONFIG"
  set +a
  [[ ${DATABASE_URL:-} =~ ^postgres(ql)?://[^/]+@(127\.0\.0\.1|localhost):5432/redpacket$ ]] || fail "Expected local redpacket database URL on port 5432; remote/custom databases require a separate deployment configuration."
  [[ ${SERVER_HOST:-} == 127.0.0.1 && ${SERVER_PORT:-} == 8080 ]] || fail "Deployment expects 127.0.0.1:8080."
}
require_rust() {
  [[ -x $CARGO/cargo && -x $CARGO/rustup ]] || fail "Missing rustup/Cargo; run 01-install-env.sh rust."
  as_build "$CARGO/rustup" run stable cargo --version >/dev/null || fail "Missing stable Rust; run 01-install-env.sh rust."
}
