# Build stage
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "pub fn lib() {}" > src/lib.rs

# Build dependencies only
RUN cargo build --release --bin book-generator-server 2>/dev/null || true

# Remove dummy files
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the server binary
RUN cargo build --release --bin book-generator-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies including pandoc and texlive for PDF/EPUB generation
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    pandoc \
    texlive-xetex \
    texlive-fonts-recommended \
    texlive-fonts-extra \
    fonts-liberation \
    fonts-dejavu \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create data directory for book output
RUN mkdir -p /data/books

# Copy the binary
COPY --from=builder /app/target/release/book-generator-server /app/book-generator-server

# Set environment variables
ENV PORT=8081
ENV OUTPUT_BASE=/data/books
ENV RUST_LOG=book_generator_server=info,tower_http=info

EXPOSE 8081

CMD ["/app/book-generator-server"]
