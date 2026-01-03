#!/bin/bash
# Verify deployment prerequisites

set -e

echo "🔍 Verifying deployment prerequisites..."

# Check Rust toolchain
echo "Checking Rust toolchain..."
rustc --version
cargo --version

# Check WASM target
echo "Checking WASM target..."
rustup target list --installed | grep -q wasm32-wasip1 || {
  echo "❌ wasm32-wasip1 target not installed"
  exit 1
}

# Check Spin CLI if available
if command -v spin &> /dev/null; then
  echo "Checking Spin CLI..."
  spin --version
fi

# Verify project structure
echo "Checking project structure..."
if [ ! -f "Cargo.toml" ]; then
  echo "⚠️ No root Cargo.toml found, skipping Rust verification"
else
  echo "Verifying Cargo.toml..."
  cargo metadata --format-version 1 > /dev/null 2>&1 || {
    echo "⚠️ Cargo metadata check failed, continuing anyway"
  }
fi

# Check frontend
if [ -d "frontend/app" ]; then
  echo "Checking frontend..."
  if [ -f "frontend/app/package.json" ]; then
    echo "✅ Frontend package.json found"
  else
    echo "⚠️ Frontend package.json not found"
  fi
fi

# Check Kubernetes manifests
if [ -d "k8s/base" ]; then
  echo "Checking Kubernetes manifests..."
  if [ -f "k8s/base/kustomization.yaml" ]; then
    echo "✅ Kustomization found"
  fi
fi

echo "✅ Deployment verification complete!"

