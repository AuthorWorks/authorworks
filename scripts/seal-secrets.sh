#!/usr/bin/env bash
# Re-seal AuthorWorks SealedSecrets against the live cluster's sealed-secrets controller.
#
# What it produces:
#   k8s/overlays/homelab/sealed-secrets.yaml      (Secret: authorworks-secrets)
#   k8s/overlays/homelab/ghcr-sealed-secret.yaml  (Secret: ghcr-pull-secret)
#
# Plaintext is read from environment variables. Anything missing is prompted for
# interactively (silent input). Plaintext never touches disk.
#
# Required env (or interactive prompts):
#   GHCR_USERNAME         GitHub username for GHCR
#   GHCR_TOKEN            PAT with read:packages
#   DATABASE_URL          postgresql://...authorworks DB
#   LOGTO_DATABASE_URL    postgresql://...logto DB
#   REDIS_URL             redis://...
#   LOGTO_APP_ID          Logto application ID
#   LOGTO_APP_SECRET      Logto application secret
#   JWT_SECRET            random 32+ bytes (set to "generate" to auto-generate)
#   ANTHROPIC_API_KEY     optional; leave empty to omit
#
# Optional env:
#   CONTROLLER_NAMESPACE  default: kube-system
#   CONTROLLER_NAME       default: sealed-secrets-controller
#   APP_NAMESPACE         default: authorworks

set -euo pipefail

CONTROLLER_NAMESPACE="${CONTROLLER_NAMESPACE:-kube-system}"
CONTROLLER_NAME="${CONTROLLER_NAME:-sealed-secrets-controller}"
APP_NAMESPACE="${APP_NAMESPACE:-authorworks}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
OVERLAY_DIR="$REPO_ROOT/k8s/overlays/homelab"

CERT_FILE="$(mktemp -t aw-sealed-cert.XXXXXX)"
trap 'rm -f "$CERT_FILE"' EXIT

command -v kubectl >/dev/null || { echo "kubectl required" >&2; exit 1; }
command -v kubeseal >/dev/null || { echo "kubeseal required (brew install kubeseal)" >&2; exit 1; }

echo "Fetching controller cert from $CONTROLLER_NAMESPACE/$CONTROLLER_NAME..."
kubeseal --fetch-cert \
  --controller-namespace "$CONTROLLER_NAMESPACE" \
  --controller-name "$CONTROLLER_NAME" \
  > "$CERT_FILE"

prompt_silent() {
  local var="$1" desc="$2" current="${!1-}"
  if [[ -z "$current" ]]; then
    read -rsp "$desc: " current
    echo
  fi
  printf -v "$var" '%s' "$current"
}

prompt_visible() {
  local var="$1" desc="$2" current="${!1-}"
  if [[ -z "$current" ]]; then
    read -rp "$desc: " current
  fi
  printf -v "$var" '%s' "$current"
}

prompt_visible GHCR_USERNAME "GHCR username (GitHub login)"
prompt_silent  GHCR_TOKEN    "GHCR PAT (scope read:packages)"
prompt_silent  DATABASE_URL  "DATABASE_URL"
prompt_silent  LOGTO_DATABASE_URL "LOGTO_DATABASE_URL"
prompt_silent  REDIS_URL     "REDIS_URL"
prompt_visible LOGTO_APP_ID  "LOGTO_APP_ID"
prompt_silent  LOGTO_APP_SECRET "LOGTO_APP_SECRET"

if [[ "${JWT_SECRET:-}" == "generate" || -z "${JWT_SECRET:-}" ]]; then
  if [[ "${JWT_SECRET:-}" == "generate" ]]; then
    JWT_SECRET="$(openssl rand -base64 32)"
    echo "Generated JWT_SECRET (32 random bytes, base64)"
  else
    prompt_silent JWT_SECRET "JWT_SECRET (or 'generate' to auto-generate)"
    [[ "$JWT_SECRET" == "generate" ]] && JWT_SECRET="$(openssl rand -base64 32)" && echo "Generated JWT_SECRET"
  fi
fi

ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY-}"

DOCKER_CONFIG_JSON="$(printf '{"auths":{"ghcr.io":{"username":"%s","password":"%s","auth":"%s"}}}' \
  "$GHCR_USERNAME" "$GHCR_TOKEN" "$(printf '%s:%s' "$GHCR_USERNAME" "$GHCR_TOKEN" | base64 | tr -d '\n')")"

echo "Sealing ghcr-pull-secret..."
kubectl create secret docker-registry ghcr-pull-secret \
  --namespace "$APP_NAMESPACE" \
  --docker-server=ghcr.io \
  --docker-username="$GHCR_USERNAME" \
  --docker-password="$GHCR_TOKEN" \
  --dry-run=client -o yaml \
| kubeseal --cert "$CERT_FILE" --format yaml \
> "$OVERLAY_DIR/ghcr-sealed-secret.yaml"

echo "Sealing authorworks-secrets..."
{
  echo "apiVersion: v1"
  echo "kind: Secret"
  echo "metadata:"
  echo "  name: authorworks-secrets"
  echo "  namespace: $APP_NAMESPACE"
  echo "type: Opaque"
  echo "stringData:"
  printf '  database-url: %s\n'        "$(printf '%q' "$DATABASE_URL")"
  printf '  logto-database-url: %s\n'  "$(printf '%q' "$LOGTO_DATABASE_URL")"
  printf '  redis-url: %s\n'           "$(printf '%q' "$REDIS_URL")"
  printf '  jwt-secret: %s\n'          "$(printf '%q' "$JWT_SECRET")"
  printf '  logto-app-id: %s\n'        "$(printf '%q' "$LOGTO_APP_ID")"
  printf '  logto-app-secret: %s\n'    "$(printf '%q' "$LOGTO_APP_SECRET")"
  if [[ -n "$ANTHROPIC_API_KEY" ]]; then
    printf '  anthropic-api-key: %s\n' "$(printf '%q' "$ANTHROPIC_API_KEY")"
  fi
} | kubeseal --cert "$CERT_FILE" --format yaml \
> "$OVERLAY_DIR/sealed-secrets.yaml"

echo
echo "Wrote:"
echo "  $OVERLAY_DIR/sealed-secrets.yaml"
echo "  $OVERLAY_DIR/ghcr-sealed-secret.yaml"
echo
echo "Verify with:"
echo "  kubectl kustomize k8s/overlays/homelab | kubectl apply --dry-run=server -f -"
echo
echo "Commit and push; ArgoCD will sync the SealedSecrets and the controller will decrypt."
