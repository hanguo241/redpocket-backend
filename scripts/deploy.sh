#!/usr/bin/env bash
# Convenience orchestration; numbered scripts may also be run independently.
set -euo pipefail
SCRIPTS=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
usage() { echo 'Usage: sudo bash scripts/deploy.sh {install|update} [API_DOMAIN_OR_IP]'; }
case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  install)
    [[ $# -eq 2 && $2 =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]] || { usage; exit 1; }
    bash "$SCRIPTS/01-install-env.sh"
    bash "$SCRIPTS/02-init-db.sh"
    bash "$SCRIPTS/03-migrate-db.sh"
    bash "$SCRIPTS/04-build.sh"
    bash "$SCRIPTS/05-configure-services.sh"
    bash "$SCRIPTS/06-configure-nginx.sh" "$2"
    ;;
  update)
    [[ $# -eq 1 ]] || { usage; exit 1; }
    bash "$SCRIPTS/03-migrate-db.sh"
    bash "$SCRIPTS/04-build.sh"
    bash "$SCRIPTS/05-configure-services.sh"
    ;;
  *) usage; exit 1 ;;
esac
echo 'Deployment complete.'
