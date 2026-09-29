#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/00-check-env.sh"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -eq 0 ]] || fail "Unexpected arguments."
init_step
missing=0
check() {
  local label=$1
  shift
  if "$@" >/dev/null 2>&1; then
    echo "OK      $label"
  else
    echo "MISSING $label"
    missing=1
  fi
}
package_installed() {
  [[ $(dpkg-query -W -f='${Status}' "$1" 2>/dev/null) == 'install ok installed' ]]
}
for package in git curl ca-certificates build-essential pkg-config libssl-dev postgresql postgresql-contrib nginx openssl python3-bcrypt; do
  check "$package" package_installed "$package"
done
check 'Rust stable (login user)' as_build "$CARGO/rustup" run stable cargo --version
sqlx_version() { [[ $("$CARGO/sqlx" --version 2>/dev/null) == 'sqlx-cli 0.8.6' ]]; }
check 'SQLx CLI 0.8.6 (login user)' sqlx_version
check 'PostgreSQL connection (local postgres)' pg -d postgres -Atc 'SELECT 1'
check 'Python bcrypt' python3 -c 'import bcrypt'
if [[ $missing -ne 0 ]]; then
  echo 'Install only missing components, or use 01-install-env.sh [system|rust|sqlx|all].'
  exit 1
fi
echo 'Environment ready; skip 01-install-env.sh. No packages or project configuration were changed.'
