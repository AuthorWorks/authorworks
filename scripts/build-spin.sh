#!/bin/bash
set -e

echo "🔨 Building AuthorWorks Spin Application"

# Check prerequisites
command -v spin >/dev/null 2>&1 || { echo "❌ spin CLI is required but not installed. Install from https://developer.fermyon.com/spin/install" >&2; exit 1; }
command -v rustc >/dev/null 2>&1 || { echo "❌ Rust compiler is required but not installed. Install from https://rustup.rs/" >&2; exit 1; }

# Check if wasm32-wasip1 target is installed
if ! rustup target list --installed | grep -q wasm32-wasip1; then
    echo "📦 Installing wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# Set optimization level
OPTIMIZATION=${OPTIMIZATION:-"release"}
PROFILE=${PROFILE:-"release"}

echo "🏗️  Building with profile: $PROFILE"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf target/wasm32-wasip1

# Build all services
echo "🔧 Building all Spin services..."

services=(
    "authorworks-user-service"
    "authorworks-content-service" 
    "authorworks-storage-service"
    "authorworks-editor-service"
    "authorworks-messaging-service"
    "authorworks-discovery-service"
    "authorworks-audio-service"
    "authorworks-video-service"
    "authorworks-graphics-service"
    "authorworks-subscription-service"
)

for service in "${services[@]}"; do
    if [ -d "$service" ]; then
        echo "  📦 Building $service..."
        cd "$service"
        cargo build --target wasm32-wasip1 --profile "$PROFILE"
        cd ..
    else
        echo "  ⚠️  Warning: $service directory not found, skipping..."
    fi
done

# Build UI shell if it exists
if [ -d "authorworks-ui-shell" ]; then
    echo "🎨 Building UI shell..."
    cd authorworks-ui-shell
    if [ -f "package.json" ]; then
        npm install
        npm run build
    elif [ -f "Cargo.toml" ]; then
        cargo build --target wasm32-wasip1 --profile "$PROFILE"
    fi
    cd ..
fi

# Optimize WASM modules if requested
if [ "$OPTIMIZE" = "true" ]; then
    echo "⚡ Optimizing WASM modules..."
    ./scripts/optimize-wasm.sh
fi

# Build the Spin application
echo "🌟 Building Spin application..."
spin build

# Verify the build
echo "✅ Verifying build..."
if [ -f "spin.toml" ]; then
    echo "  📋 Spin manifest found"
else
    echo "  ❌ spin.toml not found!"
    exit 1
fi

# Check for WASM files
wasm_count=0
for service in "${services[@]}"; do
    wasm_file="${service}/target/wasm32-wasip1/${PROFILE}/${service//-/_}.wasm"
    if [ -f "$wasm_file" ]; then
        size=$(du -h "$wasm_file" | cut -f1)
        echo "  ✅ $service: $size"
        ((wasm_count++))
    else
        echo "  ❌ Missing: $wasm_file"
    fi
done

echo ""
echo "📊 Build Summary:"
echo "  Services built: $wasm_count/${#services[@]}"
echo "  Profile: $PROFILE"
echo "  Target: wasm32-wasip1"

if [ "$wasm_count" -eq "${#services[@]}" ]; then
    echo "✅ All services built successfully!"
    echo ""
    echo "🚀 Ready to deploy with:"
    echo "  spin up                    # Run locally"
    echo "  spin deploy                # Deploy to Fermyon Cloud"
    echo "  docker build -f Dockerfile.spin  # Build container"
else
    echo "⚠️  Some services failed to build"
    exit 1
fi

# Optional: Run tests
if [ "$RUN_TESTS" = "true" ]; then
    echo "🧪 Running tests..."
    for service in "${services[@]}"; do
        if [ -d "$service" ]; then
            echo "  Testing $service..."
            cd "$service"
            cargo test --target wasm32-wasip1 || echo "  ⚠️  Tests failed for $service"
            cd ..
        fi
    done
fi

echo "🎉 Build complete!"
