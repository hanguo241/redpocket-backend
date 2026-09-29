#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/04-build.sh [JOBS]"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -le 1 ]] || fail "Too many arguments."
JOBS=${1:-}
[[ -z $JOBS || $JOBS =~ ^[1-9][0-9]*$ ]] || fail "JOBS must be a positive integer."
init_step
require_rust
[[ -f Cargo.lock ]] || fail "Cargo.lock is required."
args=(+stable build --release --locked --bins)
[[ -z $JOBS ]] || args+=(-j "$JOBS")
as_build "$CARGO/cargo" "${args[@]}"
echo 'Build complete. Binaries are in target/release; running services have not changed.'
