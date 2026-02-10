#!/bin/bash
set -e

echo "🏠 Applying AuthorWorks to Homelab K3s cluster"
echo ""
echo "ℹ️  For GitOps with ArgoCD, use: ./bootstrap-argocd.sh"
echo "    This script is for manual/direct deployment."
echo ""

# Check prerequisites
command -v kubectl >/dev/null 2>&1 || { echo "❌ kubectl required"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "📍 Using context: $(kubectl config current-context)"
read -p "Continue? (y/n) " -n 1 -r
echo
[[ ! $REPLY =~ ^[Yy]$ ]] && exit 1

# Create namespace
echo "📁 Creating namespace..."
kubectl create namespace authorworks --dry-run=client -o yaml | kubectl apply -f -

# Check for secrets
if ! kubectl get secret authorworks-secrets -n authorworks &>/dev/null; then
    echo ""
    echo "⚠️  Secrets not found! Apply them first:"
    echo "   1. Copy k8s/overlays/homelab/secrets-manual.yaml outside repo"
    echo "   2. Fill in real values"
    echo "   3. kubectl apply -f secrets-manual.yaml"
    echo ""
    read -p "Press enter after secrets are applied..."
fi

# Apply kustomization
echo "🚀 Applying kustomization..."
kubectl apply -k "${PROJECT_ROOT}/k8s/overlays/homelab"

# Wait for deployments (base-minimal: frontend + book-generator; homelab overlay: server + content-worker)
echo "⏳ Waiting for deployments..."
for d in authorworks-frontend authorworks-book-generator authorworks-server authorworks-content-worker; do
  kubectl rollout status deployment/"$d" -n authorworks --timeout=120s 2>/dev/null || true
done

echo ""
echo "✅ Deployment complete!"
echo ""
kubectl get pods -n authorworks
echo ""
kubectl get ingress -n authorworks 2>/dev/null || true
echo ""
echo "🌐 Access (see ingress):"
echo "  - App: https://author.works (or your configured host)"
echo "  - API: https://api.author.works"
echo "  - Auth: https://auth.author.works (or auth.leopaska.xyz)"
