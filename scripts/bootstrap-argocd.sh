#!/usr/bin/env bash
# Bootstrap AuthorWorks on an existing ArgoCD/k3s cluster.
#
# Assumes the cluster-wide prerequisites in docs/CLUSTER_PREREQS.md are
# satisfied. As of 2026-04-28 on the homelab cluster, only the
# `argocd/ghcr-credentials` Secret is per-bootstrap; the others are managed
# in `l3ocifer/homelab` and already in place. Recap of what must be true:
#   - ArgoCD installed in the `argocd` namespace
#   - argocd-image-updater Deployment running v0.14.0 with
#     applications_api: kubernetes (no gRPC connection to argocd-server)
#   - `homelab` AppProject exists (sourceRepos: '*', destinations: '*')
#   - sealed-secrets-controller in kube-system
#   - dockerconfigjson Secret `ghcr-credentials` in argocd ns (read:packages PAT)
#   - external `production-apps` ApplicationSet has its `authorworks` entry
#     removed/commented (already done in l3ocifer/homelab commit 775e469)
#
# What this script does:
#   1. Pre-flight: confirm context, namespaces, sealed-secrets controller.
#   2. Re-seal SealedSecrets if they've never been sealed against this cluster's key
#      (or skip with --skip-seal). Helper: scripts/seal-secrets.sh.
#   3. kubectl apply -f k8s/argocd/app-of-apps.yaml
#   4. Watch the resulting Application to Healthy.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

SKIP_SEAL=false
WAIT_TIMEOUT="${WAIT_TIMEOUT:-300s}"
for arg in "$@"; do
  case "$arg" in
    --skip-seal) SKIP_SEAL=true ;;
    --help|-h)
      sed -n '2,30p' "$0"
      exit 0
      ;;
  esac
done

command -v kubectl >/dev/null  || { echo "kubectl required" >&2; exit 1; }
command -v kubeseal >/dev/null || { echo "kubeseal required (brew install kubeseal)" >&2; exit 1; }

ctx="$(kubectl config current-context)"
echo "Context: $ctx"
read -rp "Continue? (y/N) " ans
[[ "$ans" =~ ^[Yy]$ ]] || exit 1

echo "Pre-flight checks..."
kubectl get ns argocd >/dev/null         || { echo "argocd namespace missing" >&2; exit 1; }
kubectl -n argocd get deploy argocd-server >/dev/null \
  || { echo "argocd-server not found" >&2; exit 1; }
kubectl -n argocd get deploy argocd-image-updater >/dev/null \
  || { echo "argocd-image-updater not found (cluster prereq, see docs/CLUSTER_PREREQS.md)" >&2; exit 1; }
kubectl -n argocd get appproject homelab >/dev/null \
  || { echo "AppProject 'homelab' missing in argocd ns (cluster prereq)" >&2; exit 1; }
kubectl -n argocd get secret ghcr-credentials >/dev/null 2>&1 \
  || echo "WARNING: argocd/ghcr-credentials missing -- image-updater will fail to read GHCR until it's created (see docs/CLUSTER_PREREQS.md)"
kubectl -n kube-system get deploy sealed-secrets-controller >/dev/null \
  || { echo "sealed-secrets-controller missing in kube-system" >&2; exit 1; }

if [[ "$SKIP_SEAL" == false ]]; then
  echo
  echo "About to (re-)seal SealedSecrets in k8s/overlays/homelab/."
  echo "Plaintext is read from env vars or interactive prompts -- nothing touches git."
  read -rp "Run scripts/seal-secrets.sh now? (y/N) " ans
  if [[ "$ans" =~ ^[Yy]$ ]]; then
    "$SCRIPT_DIR/seal-secrets.sh"
  else
    echo "Skipping seal step. Make sure sealed-secrets.yaml + ghcr-sealed-secret.yaml are valid for THIS cluster before continuing."
  fi
fi

echo
echo "Applying k8s/argocd/app-of-apps.yaml ..."
kubectl apply -f "$REPO_ROOT/k8s/argocd/app-of-apps.yaml"

echo
echo "Waiting for Application authorworks-homelab to be Healthy (timeout=$WAIT_TIMEOUT)..."
kubectl -n argocd wait --for=jsonpath='{.status.health.status}'=Healthy \
  application/authorworks-homelab --timeout="$WAIT_TIMEOUT" || true

echo
echo "Status:"
kubectl -n argocd get applications -l app.kubernetes.io/name=authorworks
echo
kubectl -n authorworks get pods 2>/dev/null || true
echo
echo "Done. ArgoCD UI:"
echo "  kubectl port-forward -n argocd svc/argocd-server 8080:443"
echo "  https://localhost:8080  (user: admin, password in argocd-initial-admin-secret)"
