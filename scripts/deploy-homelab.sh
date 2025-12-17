#!/bin/bash
set -e

echo "🚀 Deploying AuthorWorks to Homelab K3s cluster with SpinKube"

# Check prerequisites
command -v kubectl >/dev/null 2>&1 || { echo "kubectl is required but not installed. Aborting." >&2; exit 1; }
command -v helm >/dev/null 2>&1 || { echo "helm is required but not installed. Aborting." >&2; exit 1; }
command -v spin >/dev/null 2>&1 || { echo "spin CLI is required but not installed. Aborting." >&2; exit 1; }

# Set variables
CLUSTER_CONTEXT=${CLUSTER_CONTEXT:-"k3s-homelab"}
NAMESPACE=${NAMESPACE:-"authorworks"}

echo "📦 Installing SpinKube operator..."
helm repo add spinkube https://spinkube.github.io/charts
helm repo update

# Install Spin operator
helm upgrade --install spin-operator spinkube/spin-operator \
  --namespace spin-system \
  --create-namespace \
  --version 0.2.0

# Install containerd-shim-spin
helm upgrade --install containerd-shim-spin spinkube/containerd-shim-spin-installer \
  --namespace spin-system \
  --version 0.14.1

echo "⏳ Waiting for Spin operator to be ready..."
kubectl wait --for=condition=available --timeout=300s \
  deployment/spin-operator-controller-manager \
  -n spin-system

echo "🔨 Building Spin application..."
spin build

echo "📤 Pushing Spin app to registry..."
# Use GitHub Container Registry for homelab deployment
REGISTRY=${REGISTRY:-"ghcr.io/authorworks"}
IMAGE_TAG=${IMAGE_TAG:-"latest"}
spin registry push ${REGISTRY}/authorworks-platform:${IMAGE_TAG}

echo "🚀 Deploying AuthorWorks application..."
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/spinapp.yaml
kubectl apply -f k8s/services.yaml
kubectl apply -f k8s/ingress.yaml
kubectl apply -f k8s/hpa.yaml

echo "⏳ Waiting for SpinApp to be ready..."
kubectl wait --for=condition=ready --timeout=300s \
  spinapp/authorworks-platform \
  -n authorworks

echo "✅ Deployment complete!"
echo ""
echo "📊 Application status:"
kubectl get spinapp -n authorworks
echo ""
echo "🌐 Access the application at:"
echo "  - Main: https://author.works"
echo "  - API: https://api.author.works"
echo "  - Auth: https://auth.author.works"
echo "  - Tenant 1: https://tenant1.author.works"
echo "  - Tenant 2: https://tenant2.author.works"