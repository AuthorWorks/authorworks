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
#   LOGTO_APP_ID          Logto application ID (use placeholder if not yet set up)
#   LOGTO_APP_SECRET      Logto application secret (use placeholder if not yet set up)
#
# Optional env:
#   ANTHROPIC_API_KEY     omitted from Secret if empty
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

command -v kubectl >/dev/null  || { echo "kubectl required" >&2; exit 1; }
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
prompt_visible LOGTO_APP_ID  "LOGTO_APP_ID (placeholder OK)"
prompt_silent  LOGTO_APP_SECRET "LOGTO_APP_SECRET (placeholder OK)"

ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY-}"

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
SECRET_YAML_TMP="$(mktemp -t aw-secret.XXXXXX.yaml)"
trap 'rm -f "$CERT_FILE" "$SECRET_YAML_TMP"' EXIT
{
  literals=(
    "--from-literal=database-url=$DATABASE_URL"
    "--from-literal=logto-app-id=$LOGTO_APP_ID"
    "--from-literal=logto-app-secret=$LOGTO_APP_SECRET"
  )
  if [[ -n "$ANTHROPIC_API_KEY" ]]; then
    literals+=("--from-literal=anthropic-api-key=$ANTHROPIC_API_KEY")
  fi
  kubectl create secret generic authorworks-secrets \
    --namespace "$APP_NAMESPACE" \
    "${literals[@]}" \
    --dry-run=client -o yaml
} > "$SECRET_YAML_TMP"

kubeseal --cert "$CERT_FILE" --format yaml \
  < "$SECRET_YAML_TMP" \
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
