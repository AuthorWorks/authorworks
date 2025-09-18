# Multi-stage build for Spin WebAssembly application
FROM rust:1.75 AS builder

# Install wasm32-wasi target
RUN rustup target add wasm32-wasi

# Install Spin CLI
RUN curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash && \
    mv /root/.spin/bin/spin /usr/local/bin/

WORKDIR /app

# Copy all service directories
COPY authorworks-user-service ./authorworks-user-service
COPY authorworks-content-service ./authorworks-content-service
COPY authorworks-storage-service ./authorworks-storage-service
COPY authorworks-editor-service ./authorworks-editor-service
COPY authorworks-messaging-service ./authorworks-messaging-service
COPY authorworks-discovery-service ./authorworks-discovery-service
COPY authorworks-audio-service ./authorworks-audio-service
COPY authorworks-video-service ./authorworks-video-service
COPY authorworks-graphics-service ./authorworks-graphics-service
COPY authorworks-subscription-service ./authorworks-subscription-service
COPY authorworks-ui-shell ./authorworks-ui-shell
COPY spin.toml ./spin.toml

# Build all Rust services for wasm32-wasi
RUN cd authorworks-user-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-content-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-storage-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-editor-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-messaging-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-discovery-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-audio-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-video-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-graphics-service && cargo build --target wasm32-wasi --release
RUN cd authorworks-subscription-service && cargo build --target wasm32-wasi --release

# Build UI if needed
RUN if [ -f "authorworks-ui-shell/package.json" ]; then \
        cd authorworks-ui-shell && \
        npm install && \
        npm run build; \
    fi

# Final stage - minimal image with Spin runtime
FROM cgr.dev/chainguard/wolfi-base:latest

# Install Spin runtime
RUN apk add --no-cache curl && \
    curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash && \
    mv /root/.spin/bin/spin /usr/local/bin/ && \
    apk del curl

WORKDIR /app

# Copy built WASM modules
COPY --from=builder /app/authorworks-user-service/target/wasm32-wasi/release/*.wasm ./authorworks-user-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-content-service/target/wasm32-wasi/release/*.wasm ./authorworks-content-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-storage-service/target/wasm32-wasi/release/*.wasm ./authorworks-storage-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-editor-service/target/wasm32-wasi/release/*.wasm ./authorworks-editor-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-messaging-service/target/wasm32-wasi/release/*.wasm ./authorworks-messaging-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-discovery-service/target/wasm32-wasi/release/*.wasm ./authorworks-discovery-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-audio-service/target/wasm32-wasi/release/*.wasm ./authorworks-audio-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-video-service/target/wasm32-wasi/release/*.wasm ./authorworks-video-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-graphics-service/target/wasm32-wasi/release/*.wasm ./authorworks-graphics-service/target/wasm32-wasi/release/
COPY --from=builder /app/authorworks-subscription-service/target/wasm32-wasi/release/*.wasm ./authorworks-subscription-service/target/wasm32-wasi/release/

# Copy UI dist if it exists
COPY --from=builder /app/authorworks-ui-shell/dist ./authorworks-ui-shell/dist

# Copy Spin manifest
COPY --from=builder /app/spin.toml ./spin.toml

EXPOSE 80

# Run Spin application
ENTRYPOINT ["spin", "up", "--listen", "0.0.0.0:80"]