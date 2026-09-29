#!/usr/bin/env bash
set -euo pipefail
set +x
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: sudo bash scripts/05-configure-services.sh"
  exit 0
fi
# shellcheck source=deploy-common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/deploy-common.sh"
[[ $# -eq 0 ]] || fail "Unexpected arguments."
init_step
require_commands systemctl curl
load_config
[[ ! -f /etc/redpacket/initializing ]] || fail "Run 03-migrate-db.sh to finish initialization."
unset DATABASE_URL SIGNER_PRIVATE_KEY RELAYER_PRIVATE_KEY JWT_SECRET SHARE_URL_HOST
for binary in redpacket-backend redpacket-sync-worker; do
  [[ -x target/release/$binary ]] || fail "Missing $binary; run 04-build.sh first."
done
id redpacket >/dev/null 2>&1 || useradd --system --home-dir /opt/redpacket-backend --shell /usr/sbin/nologin redpacket
install -d -m 755 /opt/redpacket-backend /opt/redpacket-backend/bin
for binary in redpacket-backend redpacket-sync-worker; do
  install -m 755 "target/release/$binary" "/opt/redpacket-backend/bin/$binary.new"
  mv -f "/opt/redpacket-backend/bin/$binary.new" "/opt/redpacket-backend/bin/$binary"
done


for service in redpacket-backend redpacket-sync-worker; do
  install -m 644 "$ROOT/scripts/$service.service" "/etc/systemd/system/$service.service"
done
systemctl daemon-reload
systemctl enable redpacket-backend redpacket-sync-worker
systemctl restart redpacket-backend redpacket-sync-worker
healthy=false
for ((attempt=0; attempt<30; attempt++)); do
  if curl -fsS http://127.0.0.1:8080/health >/dev/null; then healthy=true; break; fi
  sleep 2
done
[[ $healthy == true ]] || fail "API health check failed; use journalctl -u redpacket-backend."
systemctl is-active --quiet redpacket-backend redpacket-sync-worker
echo 'API and worker installed, restarted and active.'
