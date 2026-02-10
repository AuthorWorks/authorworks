# AuthorWorks Content Worker
# Handles async AI content generation tasks from RabbitMQ

FROM rust:1.83-slim-bookworm as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files (Cargo.lock optional for CI when not committed)
COPY Cargo.toml ./
COPY core ./core
COPY workers/content ./workers/content
RUN cargo generate-lockfile 2>/dev/null || true

# Build the worker
WORKDIR /app/workers/content
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/content-worker /app/content-worker

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/content-worker"]

