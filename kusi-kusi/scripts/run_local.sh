#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
export KUSI_LOCAL_MODE=true KUSI_CREATIVE_SIMULATION=true KUSI_CREATIVE_WORKER=true KUSI_OAUTH_REFRESH_WORKER=false
export BIND_ADDRESS="${BIND_ADDRESS:-127.0.0.1:8080}"
export KUSI_LOCAL_ORIGIN=http://127.0.0.1:3000
export KUSI_API_ORIGIN="http://$BIND_ADDRESS" NEXT_TELEMETRY_DISABLED=1
if [[ ! -d web/admin/node_modules ]]; then npm ci --prefix web/admin; fi
npm run build --prefix web/admin
cargo build --workspace
target/debug/kusi-server &
api_pid=$!
cleanup() { kill "$api_pid" 2>/dev/null || true; }
trap cleanup EXIT INT TERM
for attempt in {1..50}; do
  if ! kill -0 "$api_pid" 2>/dev/null; then wait "$api_pid"; exit 1; fi
  if curl --fail --silent "$KUSI_API_ORIGIN/health/ready" >/dev/null; then break; fi
  sleep .2
done
npm run start --prefix web/admin
