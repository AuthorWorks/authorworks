#!/bin/bash
set -e

echo "🚀 Bootstrapping ArgoCD for AuthorWorks on K3s"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Check prerequisites
command -v kubectl >/dev/null 2>&1 || { echo "❌ kubectl required"; exit 1; }
command -v helm >/dev/null 2>&1 || { echo "❌ helm required"; exit 1; }

echo "📍 Current context: $(kubectl config current-context)"
read -p "Continue? (y/n) " -n 1 -r
echo
[[ ! $REPLY =~ ^[Yy]$ ]] && exit 1

# Install ArgoCD
echo "📦 Installing ArgoCD..."
kubectl create namespace argocd --dry-run=client -o yaml | kubectl apply -f -

kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

echo "⏳ Waiting for ArgoCD to be ready..."
kubectl wait --for=condition=available --timeout=300s \
  deployment/argocd-server -n argocd

# Get initial admin password
echo ""
echo "🔐 ArgoCD Initial Admin Password:"
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d
echo ""
echo ""

# Create namespace and apply secrets first
echo "📁 Creating authorworks namespace..."
kubectl create namespace authorworks --dry-run=client -o yaml | kubectl apply -f -

echo ""
echo "⚠️  IMPORTANT: Apply secrets before continuing!"
echo "   1. Copy k8s/overlays/homelab/secrets-manual.yaml outside the repo"
echo "   2. Fill in actual values"
echo "   3. Run: kubectl apply -f secrets-manual.yaml"
echo ""
read -p "Press enter after secrets are applied..."

# Apply the App of Apps
echo "🎯 Deploying App of Apps..."
kubectl apply -f "${PROJECT_ROOT}/k8s/argocd/project.yaml"
kubectl apply -f "${PROJECT_ROOT}/k8s/argocd/app-of-apps.yaml"

echo ""
echo "✅ ArgoCD bootstrap complete!"
echo ""
echo "📊 Access ArgoCD UI:"
echo "   kubectl port-forward svc/argocd-server -n argocd 8080:443"
echo "   Open: https://localhost:8080"
echo "   User: admin"
echo "   Pass: (shown above)"
echo ""
echo "🌐 AuthorWorks will be available at:"
echo "   - https://author.works"
echo "   - https://api.author.works"
echo "   - https://auth.author.works"
