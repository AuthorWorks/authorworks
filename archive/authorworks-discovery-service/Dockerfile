FROM rust:1.80-slim as chef
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.80-slim as builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
COPY --from=chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/authorworks-discovery-service /app/authorworks-discovery-service
EXPOSE 3007
ENV HOST=0.0.0.0
ENV SERVICE_PORT=3007
RUN useradd -u 10001 -r -s /sbin/nologin appuser && chown -R 10001:10001 /app
USER 10001:10001
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD ["/bin/sh","-c","curl -fsS http://127.0.0.1:3007/health >/dev/null || exit 1"]
ENTRYPOINT ["/app/authorworks-discovery-service"]

