#!/bin/bash
# Seal GHCR Pull Secret for AuthorWorks
#
# This script creates a SealedSecret for pulling images from ghcr.io
# Requires: kubectl, kubeseal, and access to the cluster with sealed-secrets controller

set -e

# Configuration
NAMESPACE="authorworks"
SECRET_NAME="ghcr-pull-secret"
OUTPUT_FILE="k8s/overlays/homelab/ghcr-sealed-secret.yaml"

# Check required tools
command -v kubectl >/dev/null 2>&1 || { echo "❌ kubectl is required but not installed."; exit 1; }
command -v kubeseal >/dev/null 2>&1 || { echo "❌ kubeseal is required but not installed."; exit 1; }

# Prompt for credentials
echo "🔐 GHCR Pull Secret Setup for AuthorWorks"
echo "==========================================="
echo ""
read -p "GitHub Username: " GITHUB_USERNAME
read -sp "GitHub Personal Access Token (with read:packages scope): " GITHUB_TOKEN
echo ""
read -p "Email: " GITHUB_EMAIL

# Detect sealed-secrets controller namespace
SEALED_SECRETS_NS=$(kubectl get deployment -A -l app.kubernetes.io/name=sealed-secrets -o jsonpath='{.items[0].metadata.namespace}' 2>/dev/null || echo "sealed-secrets")
SEALED_SECRETS_NAME=$(kubectl get deployment -n "$SEALED_SECRETS_NS" -l app.kubernetes.io/name=sealed-secrets -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "sealed-secrets-controller")

echo ""
echo "📦 Using sealed-secrets controller: $SEALED_SECRETS_NAME in namespace: $SEALED_SECRETS_NS"

# Create temporary secret
TEMP_SECRET=$(mktemp)
kubectl create secret docker-registry "$SECRET_NAME" \
  --namespace="$NAMESPACE" \
  --docker-server=ghcr.io \
  --docker-username="$GITHUB_USERNAME" \
  --docker-password="$GITHUB_TOKEN" \
  --docker-email="$GITHUB_EMAIL" \
  --dry-run=client -o yaml > "$TEMP_SECRET"

# Seal the secret
echo "🔒 Sealing secret..."
kubeseal \
  --controller-name="$SEALED_SECRETS_NAME" \
  --controller-namespace="$SEALED_SECRETS_NS" \
  --format=yaml < "$TEMP_SECRET" > "$OUTPUT_FILE"

# Clean up
rm -f "$TEMP_SECRET"

echo ""
echo "✅ Sealed secret created: $OUTPUT_FILE"
echo ""
echo "Next steps:"
echo "  1. Review the generated file"
echo "  2. Commit and push: git add $OUTPUT_FILE && git commit -m 'feat: Add GHCR pull secret' && git push"
echo "  3. ArgoCD will automatically sync and deploy the secret"

