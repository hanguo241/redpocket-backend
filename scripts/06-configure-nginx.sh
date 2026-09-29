#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/06-configure-nginx.sh API_DOMAIN_OR_IP"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -eq 1 ]] || fail "Supply an API domain or IPv4 address."
DOMAIN=$1
[[ $DOMAIN =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]] || fail "Supply a domain or IPv4 address without scheme/path."
init_step
require_commands nginx systemctl
[[ -d /etc/nginx/sites-available && -d /etc/nginx/sites-enabled ]] || fail "Ubuntu Nginx site directories are missing."
if [[ ! -e /etc/nginx/sites-available/redpacket-backend ]]; then
  sed "s/__SERVER_NAME__/$DOMAIN/g" "$ROOT/scripts/redpacket-backend.nginx" > /etc/nginx/sites-available/redpacket-backend
fi
ln -sfn /etc/nginx/sites-available/redpacket-backend /etc/nginx/sites-enabled/redpacket-backend
nginx -t
systemctl enable --now nginx
systemctl reload nginx
echo 'Nginx configured; existing site/TLS configuration preserved.'
