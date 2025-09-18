#!/bin/bash
set -e

echo "🔧 Optimizing WASM modules for production deployment"

# Check for required tools
command -v wasm-opt >/dev/null 2>&1 || {
    echo "Installing wasm-opt..."
    npm install -g wasm-opt
}

command -v wasm-strip >/dev/null 2>&1 || {
    echo "Installing wabt tools..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install wabt
    else
        sudo apt-get install wabt
    fi
}

# Build all services for WASM
echo "📦 Building all services for WASM..."
cargo build --target wasm32-wasi --release --workspace

# Optimize each WASM module
echo "⚡ Optimizing WASM modules..."
for service in authorworks-*-service; do
    if [ -d "$service" ]; then
        wasm_file="$service/target/wasm32-wasi/release/${service//-/_}.wasm"
        if [ -f "$wasm_file" ]; then
            echo "  Optimizing $service..."
            
            # Get original size
            original_size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
            
            # Run wasm-opt with aggressive optimizations
            wasm-opt -Oz \
                --enable-simd \
                --enable-bulk-memory \
                --strip-debug \
                --strip-producers \
                "$wasm_file" \
                -o "${wasm_file}.opt"
            
            # Strip additional metadata
            wasm-strip "${wasm_file}.opt"
            
            # Replace original with optimized version
            mv "${wasm_file}.opt" "$wasm_file"
            
            # Get new size
            new_size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
            reduction=$((100 - (new_size * 100 / original_size)))
            
            echo "    ✅ Size reduced by ${reduction}% (${original_size} → ${new_size} bytes)"
        fi
    fi
done

echo ""
echo "📊 WASM Module Sizes:"
for service in authorworks-*-service; do
    if [ -d "$service" ]; then
        wasm_file="$service/target/wasm32-wasi/release/${service//-/_}.wasm"
        if [ -f "$wasm_file" ]; then
            size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
            size_mb=$(echo "scale=2; $size / 1048576" | bc)
            printf "  %-30s %6.2f MB\n" "$service:" "$size_mb"
        fi
    fi
done

echo ""
echo "✅ WASM optimization complete!"