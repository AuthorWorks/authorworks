# AuthorWorks Media Worker
# Handles async media processing tasks from RabbitMQ

FROM rust:1.83-slim-bookworm as builder

WORKDIR /app

# Install dependencies including FFmpeg for media processing
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY workers/media ./workers/media

# Build the worker
WORKDIR /app/workers/media
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    ffmpeg \
    imagemagick \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/media-worker /app/media-worker

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/media-worker"]

