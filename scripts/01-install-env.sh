#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/01-install-env.sh [all|system|rust|sqlx]"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -le 1 ]] || fail "Too many arguments."
COMPONENT=${1:-all}
case "$COMPONENT" in all|system|rust|sqlx) ;; *) fail "Choose all, system, rust, or sqlx." ;; esac
init_step
case "$COMPONENT" in
  all|system)
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y git curl ca-certificates build-essential pkg-config libssl-dev postgresql postgresql-contrib nginx openssl python3-bcrypt
    systemctl enable --now postgresql
    ;;
esac
case "$COMPONENT" in
  all|rust)
    require_commands curl
    if [[ ! -x $CARGO/rustup ]]; then
      installer=$(mktemp)
      curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$installer"
      chmod 644 "$installer"
      as_build sh "$installer" -y --profile minimal --default-toolchain stable
      unlink "$installer"
    fi
    as_build "$CARGO/rustup" toolchain install stable --profile minimal
    ;;
esac
case "$COMPONENT" in
  all|sqlx)
    require_rust
    if [[ ! -x $CARGO/sqlx ]] || [[ $(as_build "$CARGO/sqlx" --version) != 'sqlx-cli 0.8.6' ]]; then
      as_build "$CARGO/cargo" +stable install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls
    fi
    ;;
esac
echo "Environment component ready: $COMPONENT"
