# Multi-stage build for Rust + Spin WebAssembly service
FROM rust:1.83-slim as builder

WORKDIR /build

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Add wasm32-wasi target
RUN rustup target add wasm32-wasi

# Copy workspace Cargo files
COPY Cargo.toml ./Cargo.toml

# Copy this service
COPY . ./

# Build the WebAssembly module
RUN cargo build --target wasm32-wasi --release

# Runtime stage - use Fermyon Spin
FROM ghcr.io/fermyon/spin:v3.0.0

WORKDIR /app

# Copy the built WASM module
COPY --from=builder /build/target/wasm32-wasi/release/*.wasm ./

# Copy spin configuration if it exists
COPY spin.toml* ./ 2>/dev/null || true

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:${SERVICE_PORT:-3001}/health || exit 1

# Run with spin
CMD ["spin", "up"]
