# AuthorWorks Spin Deployment Guide

## Overview

This document provides instructions for deploying AuthorWorks services using Fermyon Spin, a framework for building and running WebAssembly microservices. This approach enables efficient deployment of services on the home server infrastructure while maintaining low resource usage.

## Why Spin for AuthorWorks

- **Resource Efficiency**: WebAssembly provides near-native performance with lower memory overhead
- **Fast Cold Start**: Services start in milliseconds rather than seconds
- **Language Flexibility**: Support for Rust, Go, JavaScript, Python, and more
- **Isolation**: Strong security boundaries between services
- **Scalability**: Easily scale services across the home server and Raspberry Pi cluster

## Prerequisites

- Fermyon Spin 2.0 or later
- Docker and Docker Compose
- Traefik for API Gateway and routing
- Redis for state management (already present in home setup)

## Installation

### Installing Spin

```bash
# Install using Homebrew
brew tap fermyon/tap
brew install fermyon/tap/spin

# Alternatively, use the install script
curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash
sudo mv spin /usr/local/bin/
```

### Verifying Installation

```bash
spin --version
```

## AuthorWorks Service Templates

We've created Spin templates for all AuthorWorks services to ensure consistency and easy setup:

### Using the Templates

```bash
# Initialize a new service from template
spin new authorworks-service my-service

# Templates are available for:
# - authorworks-http-api (HTTP API service)
# - authorworks-event-processor (Event processing service)
# - authorworks-worker (Background worker service)
```

## Service Configuration

### spin.toml Structure

Each service follows this basic structure:

```toml
spin_manifest_version = "1"
name = "authorworks-content-service"
version = "0.1.0"
description = "AuthorWorks Content Service"

[[component]]
id = "content-api"
source = "target/wasm32-wasi/release/content_api.wasm"
environment = { REDIS_ADDRESS = "redis://redis:6379" }
[component.trigger]
route = "/v1/content/..."
executor = { type = "wagi" }

[[component]]
id = "content-worker"
source = "target/wasm32-wasi/release/content_worker.wasm"
environment = { REDIS_ADDRESS = "redis://redis:6379" }
[component.trigger]
channel = "content-events"
```

### Environment Variables

Standard environment variables across services:

| Variable | Purpose | Example |
|----------|---------|---------|
| REDIS_ADDRESS | Connection to Redis | redis://redis:6379 |
| DB_CONNECTION | PostgreSQL connection | postgres://user:pass@postgres:5432/db |
| AUTH_SECRET | JWT secret key | Generated secret |
| LOG_LEVEL | Logging level | info, debug, warn, error |
| CORS_ORIGINS | Allowed origins | https://authorworks.app |

## Service-Specific Deployment

### User Service

```toml
# spin.toml
spin_manifest_version = "1"
name = "authorworks-user-service"
version = "0.1.0"
description = "AuthorWorks User Service"

[[component]]
id = "user-api"
source = "target/wasm32-wasi/release/user_api.wasm"
environment = { 
  REDIS_ADDRESS = "redis://redis:6379",
  DB_CONNECTION = "postgres://user:pass@postgres:5432/authorworks_users",
  AUTH_SECRET = "${AUTH_SECRET}",
  AUTHELIA_INTEGRATION = "true"
}
[component.trigger]
route = "/v1/users/..."
executor = { type = "wagi" }
```

### Content Service

```toml
# spin.toml
spin_manifest_version = "1"
name = "authorworks-content-service"
version = "0.1.0"
description = "AuthorWorks Content Service"

[[component]]
id = "content-api"
source = "target/wasm32-wasi/release/content_api.wasm"
environment = { 
  REDIS_ADDRESS = "redis://redis:6379",
  DB_CONNECTION = "postgres://user:pass@postgres:5432/authorworks_content",
  STORAGE_ENDPOINT = "http://minio:9000"
}
[component.trigger]
route = "/v1/content/..."
executor = { type = "wagi" }
```

### Storage Service

```toml
# spin.toml
spin_manifest_version = "1"
name = "authorworks-storage-service"
version = "0.1.0"
description = "AuthorWorks Storage Service"

[[component]]
id = "storage-api"
source = "target/wasm32-wasi/release/storage_api.wasm"
environment = { 
  REDIS_ADDRESS = "redis://redis:6379",
  DB_CONNECTION = "postgres://user:pass@postgres:5432/authorworks_storage",
  MINIO_ENDPOINT = "http://minio:9000",
  MINIO_ACCESS_KEY = "${MINIO_ACCESS_KEY}",
  MINIO_SECRET_KEY = "${MINIO_SECRET_KEY}"
}
[component.trigger]
route = "/v1/storage/..."
executor = { type = "wagi" }
```

## Building Services

### Building a Rust Service

```bash
# In service directory
rustup target add wasm32-wasi
cargo build --target wasm32-wasi --release
spin build
```

### Building a TypeScript Service

```bash
# In service directory
npm install
npm run build
spin build
```

## Deployment

### Docker Compose Integration

```yaml
# docker-compose.yml
version: '3'

services:
  traefik:
    image: traefik:v2.9
    # ... traefik configuration ...
    
  spin-runtime:
    image: fermyon/spin:v2.0
    volumes:
      - ./services:/services
    command: "spin up --listen 0.0.0.0:8080 --file /services/spin.toml"
    environment:
      - REDIS_ADDRESS=redis://redis:6379
      - DB_CONNECTION=postgres://user:pass@postgres:5432/authorworks
      - AUTH_SECRET=${AUTH_SECRET}
    depends_on:
      - redis
      - postgres
    deploy:
      resources:
        limits:
          memory: 2G
        reservations:
          memory: 1G
          
  # ... other services ...
```

### Multi-Service Configuration

For running multiple services:

```bash
# Create a merged spin.toml with all services
cat services/*/spin.toml > spin-all.toml

# Start all services
spin up --file spin-all.toml --listen 0.0.0.0:8080
```

### Resource Allocation

| Service | Memory Allocation | Scaling Strategy |
|---------|------------------|------------------|
| User Service | 200MB | Single instance |
| Content Service | 300MB | Multiple instances |
| Storage Service | 300MB | Single instance |
| Editor Service | 500MB | Multiple instances |
| Messaging Service | 200MB | Single instance |
| Graphics Service | 1GB | Worker pool |
| Audio Service | 500MB | Worker pool |
| Video Service | 1GB | Worker pool |

## Service Communication

### HTTP-based APIs

Services communicate via HTTP using the internal Docker network:

```rust
async fn call_user_service(client: &Client, user_id: &str) -> Result<User, Error> {
    let response = client
        .get(&format!("http://spin-runtime:8080/v1/users/{}", user_id))
        .send()
        .await?;
    
    let user = response.json::<User>().await?;
    Ok(user)
}
```

### Event-Based Communication

Services can publish and subscribe to events using Redis:

```rust
// Publishing events
async fn publish_event(redis: &mut Connection, event: ContentEvent) -> Result<(), Error> {
    let payload = serde_json::to_string(&event)?;
    redis.publish("content-events", payload).await?;
    Ok(())
}

// Handling events (in Spin component)
fn handle_event(event: ContentEvent) -> Result<(), Error> {
    match event.event_type {
        "content.created" => process_new_content(&event.data),
        "content.updated" => update_content_index(&event.data),
        _ => Ok(()),
    }
}
```

## Integration with Existing Infrastructure

### Traefik Integration

```yaml
# traefik.yaml
http:
  routers:
    authorworks-api:
      rule: "Host(`api.authorworks.local`)"
      service: authorworks-api
      middlewares:
        - auth-headers
  
  services:
    authorworks-api:
      loadBalancer:
        servers:
          - url: "http://spin-runtime:8080"
```

### MinIO Integration

```rust
// Storage service example
async fn upload_file(
    minio_client: &minio::Client,
    bucket: &str,
    path: &str,
    data: Vec<u8>,
) -> Result<(), Error> {
    let mut reader = Cursor::new(data);
    minio_client
        .put_object(
            bucket,
            path,
            &mut reader,
            reader.get_ref().len() as u64,
            "application/octet-stream",
        )
        .await?;
    
    Ok(())
}
```

## Monitoring and Management

### Health Checks

Implement a standard health check endpoint:

```rust
#[http_component]
fn health_check(req: Request) -> Result<Response> {
    let response = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body("{\"status\":\"ok\"}")?;
    
    Ok(response)
}
```

### Metrics Collection

Prometheus metrics using Spin's built-in metrics:

```toml
# spin.toml
[[component.options]]
metrics = true
```

### Log Collection

Configure structured logging to stdout for Loki collection:

```rust
fn setup_logging() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
}
```

## Performance Optimizations

### Tuning Tips

1. **Component Sizing**: Allocate appropriate memory per component
2. **Connection Pooling**: Use connection pools for Redis and PostgreSQL
3. **Caching Strategy**: Leverage Redis for caching frequently accessed data
4. **Batching**: Process events in batches for efficiency
5. **Warm Starts**: Implement keep-alive for important components

## Troubleshooting

### Common Issues

1. **Component Crashes**:
   - Check logs with `spin logs`
   - Verify memory allocation is sufficient
   - Test component in isolation

2. **Networking Issues**:
   - Verify Docker network connectivity
   - Check Traefik routing rules
   - Test direct component access

3. **Performance Problems**:
   - Monitor resource usage with Prometheus
   - Check for memory leaks
   - Verify connection pool configurations

## Deployment Examples

### Deploying Content Service

```bash
# Build the Content Service
cd services/content-service
cargo build --target wasm32-wasi --release
spin build

# Deploy to Spin runtime
cp spin.toml /services/content-service.toml
docker-compose restart spin-runtime
```

## Conclusion

This deployment approach using Spin provides an efficient way to run AuthorWorks services on your home server infrastructure while minimizing resource usage. The modular nature of Spin components makes it easy to distribute workloads across your hardware and maintain a scalable architecture. 