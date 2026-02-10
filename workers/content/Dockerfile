# AuthorWorks Content Worker
# Handles async AI content generation tasks from RabbitMQ

FROM rust:1.83-slim-bookworm as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy full workspace so all members exist (see .dockerignore to limit context)
COPY . .
RUN cargo generate-lockfile 2>/dev/null || true

# Build only the content-worker package (override repo default wasm32-wasi for native Linux binary)
RUN cargo build --release -p content-worker --target x86_64-unknown-linux-gnu

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/content-worker /app/content-worker

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/content-worker"]

