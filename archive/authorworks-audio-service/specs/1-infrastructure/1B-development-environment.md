# Technical Specification: 1B - Development Environment Setup

## Overview

This specification details the standardized development environment for the AuthorWorks platform. It ensures consistent, reproducible development environments across all services and team members.

## Objectives

- Create consistent development environments across all team members
- Minimize "works on my machine" problems
- Simplify onboarding of new developers
- Enable local testing of service integrations
- Provide tooling for efficient development workflows

## Requirements

### 1. Development Container

Create a standard development container with the following components:

#### Core Development Tools

- Rust toolchain:
  - Stable channel (minimum version 1.75)
  - Nightly channel for specific features (minimum version 1.77-nightly)
  - Rustfmt and Clippy for code quality
  - Cargo-watch for development
  - Cargo-edit for dependency management
  - Cargo-expand for macro debugging

- Node.js environment (for Plate.js integration):
  - Node.js 20.x LTS
  - npm 10.x
  - TypeScript 5.x
  - Webpack for bundling
  - ESLint and Prettier for code quality

#### Database Tools

- PostgreSQL client tools (minimum version 16)
- SQLx CLI for database migrations
- Database visualization tools

#### Cloud & Infrastructure Tools

- AWS CLI
- Terraform CLI
- kubectl for Kubernetes management
- Docker and Docker Compose

#### Development Utilities

- Git with LFS support
- curl, wget, jq for API testing
- httpie for API interaction
- ripgrep and fd-find for code search
- tmux for terminal multiplexing
- zsh with developer-friendly configuration

### 2. Container Configuration

The development container should be configured with:

```dockerfile
FROM rust:1.75-bullseye as rust-base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    postgresql-client \
    libpq-dev \
    pkg-config \
    libssl-dev \
    nodejs \
    npm \
    git \
    curl \
    wget \
    ripgrep \
    fd-find \
    jq \
    tmux \
    zsh \
    && rm -rf /var/lib/apt/lists/*

# Install Rust components
RUN rustup component add rustfmt clippy && \
    rustup toolchain install nightly && \
    rustup component add --toolchain nightly rustfmt

# Install Rust utilities
RUN cargo install cargo-watch cargo-edit cargo-expand sqlx-cli

# Install Node.js tooling
RUN npm install -g typescript webpack webpack-cli eslint prettier

# Setup development environment
RUN curl -L https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh | sh
COPY .zshrc /root/.zshrc

# Set working directory
WORKDIR /workspace

# Default command
CMD ["zsh"]
```

### 3. Local Development Environment

Create a `docker-compose.yml` file for local development that includes:

```yaml
version: '3.8'
services:
  # PostgreSQL databases for services
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: development_only
      POSTGRES_USER: authorworks
    volumes:
      - ./init-scripts:/docker-entrypoint-initdb.d
      - postgres-data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U authorworks"]
      interval: 5s
      timeout: 5s
      retries: 5

  # S3-compatible storage
  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    volumes:
      - minio-data:/data
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Matrix server for messaging development
  synapse:
    image: matrixdotorg/synapse
    volumes:
      - ./synapse-data:/data
    ports:
      - "8008:8008"
    environment:
      SYNAPSE_SERVER_NAME: authorworks.local
      SYNAPSE_REPORT_STATS: "no"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8008/health"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Distributed tracing
  jaeger:
    image: jaegertracing/all-in-one
    ports:
      - "16686:16686"  # UI
      - "4317:4317"    # OTLP gRPC
      - "4318:4318"    # OTLP HTTP
    environment:
      COLLECTOR_OTLP_ENABLED: "true"

  # Metrics collection
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus:/etc/prometheus
    ports:
      - "9090:9090"
    command:
      - --config.file=/etc/prometheus/prometheus.yml
      - --storage.tsdb.path=/prometheus
      - --web.console.libraries=/usr/share/prometheus/console_libraries
      - --web.console.templates=/usr/share/prometheus/consoles

  # Metrics visualization
  grafana:
    image: grafana/grafana
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
    ports:
      - "3000:3000"
    depends_on:
      - prometheus
    environment:
      GF_SECURITY_ADMIN_PASSWORD: admin
      GF_USERS_ALLOW_SIGN_UP: "false"

  # API gateway for local services
  traefik:
    image: traefik:v2.10
    command:
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
    ports:
      - "80:80"
      - "8080:8080"  # Dashboard
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./traefik:/etc/traefik

  # Development container
  devcontainer:
    build:
      context: .
      dockerfile: Dockerfile.dev
    volumes:
      - .:/workspace
      - cargo-cache:/usr/local/cargo/registry
      - target-cache:/workspace/target
    depends_on:
      - postgres
      - minio
      - synapse
    environment:
      DATABASE_URL: postgres://authorworks:development_only@postgres:5432/authorworks
      MINIO_ENDPOINT: http://minio:9000
      MINIO_ACCESS_KEY: minioadmin
      MINIO_SECRET_KEY: minioadmin
      MATRIX_SERVER_URL: http://synapse:8008
      RUST_LOG: debug
      CARGO_TARGET_DIR: /workspace/target
    tty: true

volumes:
  postgres-data:
  minio-data:
  grafana-data:
  cargo-cache:
  target-cache:
```

### 4. Database Initialization

Create database initialization scripts that:

1. Create separate databases for each service
2. Set up appropriate users and permissions
3. Create extension dependencies
4. Initialize schema versions

Example initialization script:

```sql
-- Create databases for each service
CREATE DATABASE users;
CREATE DATABASE content;
CREATE DATABASE editor;
CREATE DATABASE storage;
CREATE DATABASE messaging;
CREATE DATABASE payments;
CREATE DATABASE discovery;

-- Create service-specific users with appropriate permissions
CREATE USER users_service WITH PASSWORD 'development_only';
CREATE USER content_service WITH PASSWORD 'development_only';
CREATE USER editor_service WITH PASSWORD 'development_only';
CREATE USER storage_service WITH PASSWORD 'development_only';
CREATE USER messaging_service WITH PASSWORD 'development_only';
CREATE USER payments_service WITH PASSWORD 'development_only';
CREATE USER discovery_service WITH PASSWORD 'development_only';

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE users TO users_service;
GRANT ALL PRIVILEGES ON DATABASE content TO content_service;
GRANT ALL PRIVILEGES ON DATABASE editor TO editor_service;
GRANT ALL PRIVILEGES ON DATABASE storage TO storage_service;
GRANT ALL PRIVILEGES ON DATABASE messaging TO messaging_service;
GRANT ALL PRIVILEGES ON DATABASE payments TO payments_service;
GRANT ALL PRIVILEGES ON DATABASE discovery TO discovery_service;

-- Connect to each database and set up extensions
\c users
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

\c content
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

\c payments
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

\c discovery
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
```

### 5. Development Tools and Scripts

Develop utilities to streamline the development workflow:

#### Service Runner

Create a script that can start specific services for development:

```bash
#!/bin/bash
# run-services.sh - Start specific services for development

SERVICES=$@
if [ -z "$SERVICES" ]; then
    echo "Usage: run-services.sh [service1] [service2] ..."
    echo "Available services: postgres minio synapse jaeger prometheus grafana traefik all"
    exit 1
fi

if [[ "$SERVICES" == *"all"* ]]; then
    docker-compose up -d
else
    docker-compose up -d $SERVICES
fi

echo "Services started: $SERVICES"
```

#### Database Reset

Create a script to reset databases to a clean state:

```bash
#!/bin/bash
# reset-db.sh - Reset databases to a clean state

SERVICE=$1
if [ -z "$SERVICE" ]; then
    echo "Usage: reset-db.sh [service-name]"
    echo "Available services: users content editor storage messaging payments discovery all"
    exit 1
fi

if [[ "$SERVICE" == "all" ]]; then
    # Reset all databases
    for db in users content editor storage messaging payments discovery; do
        echo "Resetting $db database..."
        PGPASSWORD=development_only psql -h localhost -U authorworks -d $db -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
        PGPASSWORD=development_only psql -h localhost -U authorworks -d $db -c "GRANT ALL ON SCHEMA public TO $db_service;"
    done
else
    # Reset specific database
    echo "Resetting $SERVICE database..."
    PGPASSWORD=development_only psql -h localhost -U authorworks -d $SERVICE -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
    PGPASSWORD=development_only psql -h localhost -U authorworks -d $SERVICE -c "GRANT ALL ON SCHEMA public TO ${SERVICE}_service;"
fi

echo "Database reset complete"
```

#### JWT Key Generation

Script to generate JWT keys for development:

```bash
#!/bin/bash
# generate-jwt-keys.sh - Generate JWT keys for development

mkdir -p .keys

# Generate private key
openssl genpkey -algorithm RSA -out .keys/jwt_private.pem -pkeyopt rsa_keygen_bits:2048

# Generate public key
openssl rsa -pubout -in .keys/jwt_private.pem -out .keys/jwt_public.pem

echo "JWT keys generated in .keys/"
```

### 6. Local HTTPS Setup

Create a script to generate development certificates for HTTPS:

```bash
#!/bin/bash
# generate-certs.sh - Generate development certificates

mkdir -p .certs

# Generate CA key and certificate
openssl genrsa -out .certs/ca.key 4096
openssl req -new -x509 -key .certs/ca.key -out .certs/ca.crt -days 365 -subj "/CN=AuthorWorks Development CA"

# Generate server key
openssl genrsa -out .certs/server.key 2048

# Generate server CSR
openssl req -new -key .certs/server.key -out .certs/server.csr -subj "/CN=*.authorworks.local"

# Generate server certificate
openssl x509 -req -in .certs/server.csr -CA .certs/ca.crt -CAkey .certs/ca.key -CAcreateserial \
    -out .certs/server.crt -days 365 -sha256 \
    -extfile <(echo "subjectAltName=DNS:*.authorworks.local,DNS:authorworks.local,DNS:localhost")

echo "Certificates generated in .certs/"
echo "To use these certificates, install .certs/ca.crt as a trusted CA in your browser/system"
```

## Implementation Steps

1. Create Dockerfile for development container
2. Set up docker-compose.yml with all required services
3. Write database initialization scripts
4. Develop utility scripts for common tasks
5. Create documentation for environment setup
6. Test the environment with sample services
7. Distribute environment setup to all team members

## Technical Decisions

### Why Docker for Development?

Docker was chosen for development environments to:
- Ensure consistency across different developer machines
- Simplify dependency management
- Isolate development environments
- Enable easy setup and teardown
- Facilitate service integration testing

### Why Include Observability Tools?

Including Jaeger, Prometheus, and Grafana in the development environment:
- Encourages developers to instrument code from the beginning
- Allows testing of observability features locally
- Helps identify performance issues early
- Creates familiarity with production observability tools

## Success Criteria

The development environment will be considered successfully implemented when:

1. All team members can set up identical development environments
2. All services can be developed, built, and tested locally
3. Integration testing can be performed in the local environment
4. New team members can be onboarded quickly
5. Development tools are standardized across the team 