#!/usr/bin/env bash
set -euo pipefail

# Simple verification script for AuthorWorks homelab deployment
# Requires docker compose and access to the same environment variables used for homelab overlay

COMPOSE=${COMPOSE:-docker compose}
FILE=${FILE:-docker-compose.homelab.yml}

echo "[verify] Checking containers are healthy..."
$COMPOSE -f "$FILE" ps

check_url() {
  local url=$1
  echo "[verify] GET $url"
  if command -v curl >/dev/null 2>&1; then
    curl -fsS "$url" | head -c 200 && echo
  else
    echo "curl not installed; skipping $url"
  fi
}

API_HOST=${API_HOST:-aw-api.${DOMAIN:-localhost}}
SCHEME=${SCHEME:-https}

check_url "$SCHEME://$API_HOST/health"
check_url "$SCHEME://$API_HOST/api/users/health" || true
check_url "$SCHEME://$API_HOST/api/content/health" || true
check_url "$SCHEME://$API_HOST/api/storage/health" || true
check_url "$SCHEME://$API_HOST/api/editor/health" || true

echo "[verify] Done."
