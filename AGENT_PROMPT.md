# AuthorWorks - Agent Task Prompt

**Version:** 1.1
**Updated:** January 2025
**Status:** Infrastructure Complete - Application Implementation Pending
**Note:** Repository structure updated - individual service repos archived, consolidated to main repo + authorworks-engine

---

## Project Overview

**AuthorWorks** is an AI-powered creative content generation platform that enables users to create, edit, and publish high-quality long-form creative content (novels, screenplays, plays) using context-aware AI generation. The platform supports collaborative editing, multi-format publishing, and monetization through subscriptions.

### Core Concept

**"AI-assisted creative content generation platform"** - A comprehensive platform that enables:
- **Content Creation** - Generate long-form creative content using Anthropic Claude API
- **Rich Text Editing** - Integrated Plate.js editor with real-time collaboration
- **Multi-Format Publishing** - Transform content into novels, graphic novels, audio, video
- **Community & Discovery** - Content discovery, search, and recommendations
- **Monetization** - Subscription tiers and payment processing via Stripe

### Mission

Create a production-grade creative platform that combines:
- **AI-Powered Generation** - Context-aware story generation with Anthropic Claude
- **Collaborative Editing** - Real-time editing with version control
- **Multi-Media Transformation** - Convert text to audio, video, graphic novels
- **Enterprise Infrastructure** - Microservices architecture with Docker/Kubernetes
- **Scalable Deployment** - Support for both Docker and Spin/WASM deployments

---

## System Architecture

### Infrastructure Foundation

**Deployment Environment:**
- **Server**: "Aleph" homelab (192.168.1.200, Ubuntu 24.04, kernel 6.14.0-35+)
- **Network**: External bridge network `llm_network` (10.0.1.0/24)
- **PostgreSQL**: 16 (neon-postgres-leopaska:5432)
  - Database: `authorworks` with 10 tables initialized
  - Credentials: `postgres:postgresstrongpassword123`
- **Redis**: 7 (redis-nd-leopaska:6379)
  - Password: `redisstrongpassword123`
- **MinIO**: S3-compatible storage (minio-leopaska:9000)
- **Traefik**: Reverse proxy with Authelia middleware
- **Cloudflare Tunnel**: Routes *.leopaska.xyz to homelab
- **Production URL**: https://authorworks.leopaska.xyz
- **Authentication**: Authelia SSO (authelia-leopaska container)

### Current Architecture (Docker Compose)

**API Gateway** (Nginx):
- Serves Leptos WASM frontend
- Routes API requests to backend services
- Port: 8080
- IP: 10.0.1.40

**Microservices** (Rust/Axum):
- **User Service** (Port 3001) - Authentication, profiles, subscriptions
- **Content Service** (Port 3002) - Story generation, content management
- **Storage Service** (Port 3003) - File storage, exports, MinIO integration
- **Editor Service** (Port 3004) - Rich text editing, collaboration
- **Messaging Service** (Port 3006) - Real-time communication, WebSocket
- **Discovery Service** (Port 3007) - Content search, recommendations, Qdrant
- **Subscription Service** (Port 3005) - Payment processing, Stripe
- **Audio Service** (Port 3008) - Text-to-speech transformation
- **Video Service** (Port 3009) - Text-to-video transformation
- **Graphics Service** (Port 3010) - Text-to-graphic novel transformation

**Frontend:**
- **Leptos WASM Application** - Served via Nginx
- Static assets in `authorworks-ui-shell/dist`
- Title: "author.works"

### Target Architecture (Spin/WASM - Planned)

**Spin Services** (WebAssembly):
- Gateway, Users, Content, Storage, Editor, Messaging, Discovery, Subscription, Audio, Video, Graphics
- Benefits: 95% memory reduction, sub-millisecond cold starts, true multi-tenancy

**Status**: Configuration ready, implement after Docker version working

### Technology Stack

**Backend:**
- **Language**: Rust (edition 2021)
- **Web Framework**: Axum 0.7+
- **Database**: PostgreSQL 16 with SQLx
- **Caching**: Redis 7
- **Object Storage**: MinIO (S3-compatible)
- **Message Queue**: RabbitMQ (optional)

**Frontend:**
- **Framework**: Leptos (Rust/WASM)
- **Editor**: Plate.js (planned)
- **Build Tool**: Trunk (for WASM builds)

**AI/ML:**
- **Anthropic Claude API** - Content generation
- **Qdrant** - Vector database for embeddings
- **Text-to-Speech** - Audio generation
- **Text-to-Video** - Video generation

**Infrastructure:**
- **Docker**: Containerization
- **Docker Compose**: Local development
- **Kubernetes/K3s**: Orchestration
- **Traefik**: Ingress controller
- **Authelia**: SSO authentication
- **Cloudflare Tunnel**: External access

---

## Repository Structure

**Note**: Individual service repositories have been archived. The platform now uses a consolidated architecture with the main repository and the `authorworks-engine` submodule containing specifications and core functionality.

```
authorworks/
├── authorworks-engine/              # Core engine & specifications (ACTIVE SUBMODULE)
│   ├── specs/                       # Service specifications
│   └── [engine code]
├── [Archived Service Directories]   # Kept locally for reference (ARCHIVED)
│   ├── authorworks-user-service/     # (archived)
│   ├── authorworks-content-service/ # (archived)
│   ├── authorworks-storage-service/  # (archived)
│   ├── authorworks-editor-service/  # (archived)
│   ├── authorworks-messaging-service/ # (archived)
│   ├── authorworks-discovery-service/ # (archived)
│   ├── authorworks-subscription-service/ # (archived)
│   ├── authorworks-audio-service/  # (archived)
│   ├── authorworks-video-service/  # (archived)
│   ├── authorworks-graphics-service/ # (archived)
│   ├── authorworks-ui-shell/        # (archived)
│   ├── authorworks-platform/        # (archived)
│   ├── authorworks-docs/           # (archived)
│   └── author_works/                # (archived - legacy)
├── k8s/                             # Kubernetes manifests
│   ├── namespace.yaml
│   ├── services.yaml
│   ├── ingress.yaml
│   ├── spinapp.yaml                 # Spin/WASM deployment
│   └── overlays/
├── docker-compose.homelab.yml       # Homelab deployment
├── docker-compose.yml               # Local development
├── docker-compose.production.yml    # Production deployment
├── docker-compose.spin.yml          # Spin/WASM deployment
├── nginx.conf                       # API Gateway configuration
├── init-schema.sql                  # Database schema
├── Cargo.toml                       # Workspace configuration
├── spin.toml                        # Spin application manifest
├── Makefile                         # Build automation
├── scripts/                          # Deployment scripts
│   ├── deploy-homelab.sh
│   ├── verify-homelab.sh
│   └── build-spin.sh
└── docs/                             # Documentation
    ├── DEPLOYMENT.md
    ├── SERVICE_ARCHITECTURE.md
    └── SPIN_DEPLOYMENT.md
```

**Active Repositories:**
- `authorworks` (this repo) - Main umbrella repository
- `authorworks-engine` - Core engine, specifications, and shared functionality

**Archived Repositories:**
All individual service repositories have been archived on GitHub. The service directories remain in the local repository for reference but are no longer active submodules. The platform architecture has been consolidated into the main repository and engine submodule.

---

## Core Services

### 1. User Service (Port 3001)

**Purpose**: Authentication, authorization, and user profile management

**Features:**
- User registration and authentication
- JWT token generation and validation
- Profile management
- Role-based access control
- Session management
- OAuth integration (Authelia)
- Subscription tier management

**API Endpoints:**
- `POST /api/auth/register` - User registration
- `POST /api/auth/login` - User login
- `GET /api/auth/profile` - Get user profile
- `PUT /api/users/profile` - Update profile
- `POST /api/auth/refresh` - Refresh token
- `GET /api/users/{id}` - Get user by ID

**Dependencies:**
- PostgreSQL (user data)
- Redis (sessions, caching)
- Authelia (SSO)

### 2. Content Service (Port 3002)

**Purpose**: Story creation, AI generation, and content management

**Features:**
- Context-aware story generation (Anthropic Claude)
- Book/chapter/scene management
- Character database
- Generation history tracking
- Content versioning
- Publishing workflow

**API Endpoints:**
- `POST /api/stories` - Create story
- `GET /api/stories` - List stories
- `GET /api/stories/{id}` - Get story
- `PUT /api/stories/{id}` - Update story
- `POST /api/stories/{id}/generate` - Generate content
- `POST /api/stories/{id}/publish` - Publish story
- `GET /api/chapters/{id}` - Get chapter
- `POST /api/characters` - Create character

**Dependencies:**
- PostgreSQL (content storage)
- Redis (caching)
- Anthropic Claude API
- Storage Service (file attachments)

### 3. Storage Service (Port 3003)

**Purpose**: File storage abstraction and export generation

**Features:**
- File upload/download (MinIO/S3)
- Export generation (PDF, EPUB, DOCX)
- Version storage
- Presigned URL generation
- File metadata tracking
- Storage quotas

**API Endpoints:**
- `POST /api/upload` - Upload file
- `GET /api/files/{id}` - Download file
- `DELETE /api/files/{id}` - Delete file
- `POST /api/export` - Generate export
- `GET /api/storage/usage` - Storage statistics

**Dependencies:**
- PostgreSQL (file metadata)
- MinIO/S3 (object storage)

### 4. Editor Service (Port 3004)

**Purpose**: Real-time collaborative editing and AI writing assistance

**Features:**
- Plate.js rich text editor integration
- Real-time collaboration (WebSocket)
- AI writing assistance
- Auto-save functionality
- Version control
- Editor sessions

**API Endpoints:**
- `POST /api/editor/sessions` - Create editing session
- `GET /api/editor/sessions/{id}` - Join session
- `WS /api/editor/ws` - Real-time collaboration
- `POST /api/editor/ai-assist` - AI suggestions
- `POST /api/editor/save` - Save content

**Dependencies:**
- PostgreSQL (editor sessions)
- Redis (real-time state)
- Content Service (content operations)
- Messaging Service (collaboration events)

### 5. Messaging Service (Port 3006)

**Purpose**: Event bus, notifications, and real-time communication

**Features:**
- WebSocket support
- Event publishing/subscribing
- User notifications
- Real-time collaboration events
- Message queuing (RabbitMQ)

**API Endpoints:**
- `WS /api/messaging/ws` - WebSocket connection
- `POST /api/messaging/events` - Publish event
- `GET /api/messaging/notifications` - Get notifications

**Dependencies:**
- RabbitMQ (message queuing)
- Redis (WebSocket state)

### 6. Discovery Service (Port 3007)

**Purpose**: Content discovery, search, and recommendations

**Features:**
- Vector search with Qdrant
- Content recommendations
- Full-text search
- Trending content
- Category browsing

**API Endpoints:**
- `POST /api/discovery/search` - Search content
- `GET /api/discovery/recommendations` - Get recommendations
- `GET /api/discovery/trending` - Trending content
- `GET /api/discovery/categories` - Categories

**Dependencies:**
- PostgreSQL (content metadata)
- Qdrant (vector database)
- Content Service (content data)

### 7. Subscription Service (Port 3005)

**Purpose**: Billing, payments, and subscription management

**Features:**
- Stripe integration
- Subscription management
- Payment processing
- Invoice generation
- Webhook handling

**API Endpoints:**
- `POST /api/subscriptions` - Create subscription
- `PUT /api/subscriptions/{id}` - Update subscription
- `POST /api/billing/webhook` - Stripe webhook
- `GET /api/billing/invoices` - Get invoices

**Dependencies:**
- PostgreSQL (subscription data)
- User Service (user information)
- Stripe API

### 8. Media Transformation Services

**Audio Service** (Port 3008):
- Text-to-speech transformation
- Voice synthesis
- Audio file generation

**Video Service** (Port 3009):
- Text-to-video transformation
- Video generation
- Scene composition

**Graphics Service** (Port 3010):
- Text-to-graphic novel transformation
- Image generation
- Layout composition

---

## Database Schema

### Core Tables

**Users:**
- `id` (UUID, primary key)
- `email` (unique)
- `username` (unique)
- `display_name`
- `password_hash`
- `subscription_tier` (free, basic, pro, enterprise)
- `subscription_status` (active, cancelled, expired)
- `created_at`, `updated_at`, `last_login`
- `is_active` (boolean)
- `profile_data` (JSONB)

**Books:**
- `id` (UUID, primary key)
- `user_id` (FK to users)
- `title`, `subtitle`, `genre`
- `content_type` (novel, screenplay, play, graphic_novel)
- `description`
- `cover_image_url`
- `status` (draft, in_progress, completed, published)
- `visibility` (private, unlisted, public)
- `word_count`
- `created_at`, `updated_at`, `published_at`
- `metadata`, `settings` (JSONB)

**Chapters:**
- `id` (UUID, primary key)
- `book_id` (FK to books)
- `chapter_number`
- `title`, `content`
- `word_count`
- `status` (draft, completed)
- `created_at`, `updated_at`
- `metadata` (JSONB)
- Unique constraint on (book_id, chapter_number)

**Scenes:**
- `id` (UUID, primary key)
- `chapter_id` (FK to chapters)
- `scene_number`
- `title`, `content`
- `word_count`
- `pov_character`, `setting`
- `created_at`, `updated_at`
- `metadata` (JSONB)
- Unique constraint on (chapter_id, scene_number)

**Characters:**
- `id` (UUID, primary key)
- `book_id` (FK to books)
- `name`, `role` (protagonist, antagonist, supporting, minor)
- `description`, `background`
- `traits`, `relationships` (JSONB)
- `created_at`, `updated_at`

**Generation History:**
- `id` (UUID, primary key)
- `user_id` (FK to users)
- `book_id`, `chapter_id`, `scene_id` (FKs)
- `prompt`, `generated_content`
- `model_used`
- `tokens_used`, `generation_time_ms`
- `created_at`

**Version History:**
- `id` (UUID, primary key)
- `book_id`, `chapter_id`, `scene_id` (FKs)
- `version_number`
- `content_snapshot`
- `change_summary`
- `created_at`

**Exports:**
- `id` (UUID, primary key)
- `book_id` (FK to books)
- `export_type` (PDF, EPUB, DOCX)
- `status` (pending, processing, completed, failed)
- `file_url`
- `created_at`, `completed_at`

**Collaborations:**
- `id` (UUID, primary key)
- `book_id` (FK to books)
- `user_id` (FK to users)
- `role` (owner, editor, viewer)
- `created_at`

**Storage Metadata:**
- `id` (UUID, primary key)
- `user_id` (FK to users)
- `file_path`, `file_name`
- `file_size`, `mime_type`
- `storage_provider` (minio, s3)
- `created_at`

---

## API Design Patterns

### RESTful Endpoints

**Base URL**: `https://authorworks.leopaska.xyz/api`

**Authentication:**
- `POST /api/auth/register` - Register new user
- `POST /api/auth/login` - Login (returns JWT)
- `GET /api/auth/profile` - Get user profile
- `POST /api/auth/refresh` - Refresh token
- `POST /api/auth/logout` - Logout

**Content:**
- `GET /api/stories` - List stories
- `POST /api/stories` - Create story
- `GET /api/stories/{id}` - Get story
- `PUT /api/stories/{id}` - Update story
- `DELETE /api/stories/{id}` - Delete story
- `POST /api/stories/{id}/generate` - Generate content
- `POST /api/stories/{id}/publish` - Publish story

**Storage:**
- `POST /api/upload` - Upload file
- `GET /api/files/{id}` - Download file
- `DELETE /api/files/{id}` - Delete file
- `POST /api/export` - Generate export

**Editor:**
- `POST /api/editor/sessions` - Create editing session
- `GET /api/editor/sessions/{id}` - Join session
- `WS /api/editor/ws` - Real-time collaboration
- `POST /api/editor/ai-assist` - AI suggestions

**Health:**
- `GET /health` - Health check endpoint

### Request/Response Format

**Request Headers:**
```
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Response Format:**
```json
{
  "data": { ... },
  "error": null
}
```

**Error Response:**
```json
{
  "data": null,
  "error": {
    "message": "Error description",
    "code": "ERROR_CODE"
  }
}
```

---

## Docker Configuration

### Local Development

**docker-compose.yml** includes:
- All microservices
- PostgreSQL
- Redis
- MinIO
- RabbitMQ (optional)

**docker-compose.homelab.yml** includes:
- Services on `llm_network`
- Integration with shared PostgreSQL/Redis/MinIO
- Traefik labels for routing
- Authelia middleware

### Dockerfile Pattern

**Standard Dockerfile:**
```dockerfile
FROM rust:1.89-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/service-name /app/service-name
USER 10001
EXPOSE 3001
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:3001/health || exit 1
CMD ["/app/service-name"]
```

### Building and Running

```bash
# Build all services
docker-compose -f docker-compose.homelab.yml build

# Start all services
docker-compose -f docker-compose.homelab.yml up -d

# View logs
docker-compose -f docker-compose.homelab.yml logs -f

# Check health
docker ps --filter name=authorworks
```

---

## Environment Variables

### Required Variables

**Database:**
- `DATABASE_URL`: PostgreSQL connection string
  - Format: `postgresql://user:password@host:port/database`
  - Production: `postgresql://postgres:postgresstrongpassword123@2f74558da076_neon-postgres-leopaska:5432/authorworks`

**Redis:**
- `REDIS_URL`: Redis connection string
  - Format: `redis://:password@host:port`
  - Production: `redis://:redisstrongpassword123@redis-nd-leopaska:6379`

**JWT:**
- `JWT_SECRET`: JWT signing secret (32+ chars)
  - Generate: `openssl rand -base64 32`

### Optional Variables

**Application:**
- `HOST`: Bind address (default: `0.0.0.0`)
- `SERVICE_PORT`: Service-specific port (3001-3010)
- `ALLOWED_ORIGINS`: CORS configuration
- `LOG_LEVEL`: Logging level (info, debug, warn, error)

**Anthropic Claude:**
- `ANTHROPIC_API_KEY`: Claude API key for content generation

**MinIO/S3:**
- `MINIO_ENDPOINT`: MinIO endpoint URL
- `MINIO_ACCESS_KEY`: MinIO access key
- `MINIO_SECRET_KEY`: MinIO secret key

**Stripe:**
- `STRIPE_SECRET_KEY`: Stripe secret key
- `STRIPE_WEBHOOK_SECRET`: Stripe webhook secret

**Qdrant:**
- `QDRANT_URL`: Qdrant vector database URL
- `QDRANT_COLLECTION`: Collection name

---

## Development Workflow

### Local Setup

1. **Prerequisites:**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install Docker
   docker --version
   ```

2. **Clone Repository:**
   ```bash
   git clone --recursive https://github.com/AuthorWorks/authorworks.git
   cd authorworks
   ```

3. **Environment Setup:**
   ```bash
   # Set environment variables
   export DOMAIN=leopaska.xyz
   export POSTGRES_PASSWORD=postgresstrongpassword123
   export REDIS_PASSWORD=redisstrongpassword123
   export MINIO_ROOT_USER=minioadmin
   export MINIO_ROOT_PASSWORD=minioadmin123
   export JWT_SECRET=$(openssl rand -base64 32)
   ```

4. **Database Setup:**
   ```bash
   # Initialize database schema
   docker exec 2f74558da076_neon-postgres-leopaska psql -U postgres -d authorworks -f /path/to/init-schema.sql
   ```

5. **Start Services:**
   ```bash
   docker-compose -f docker-compose.homelab.yml up -d
   ```

### Development Commands

**Makefile Commands:**
- `make build` - Build all services
- `make up` - Start all services
- `make down` - Stop all services
- `make logs` - View logs
- `make clean` - Clean up containers

**Service Development:**
```bash
# Build specific service
cd authorworks-user-service
cargo build --release

# Run service locally
cargo run

# Run tests
cargo test

# Watch for changes
cargo watch -x run
```

### Testing

**Unit Tests:**
```bash
cargo test
```

**Integration Tests:**
```bash
cargo test --test integration
```

**Health Checks:**
```bash
curl http://10.0.1.40:8080/health
curl http://10.0.1.37:3001/health  # User service
curl http://10.0.1.39:3002/health  # Content service
```

---

## Implementation Guidelines

### Code Quality Standards

1. **Rust Best Practices:**
   - Follow Rust style guide (rustfmt)
   - Pass clippy with no warnings (`-D warnings`)
   - Use `Result<T, E>` for error handling
   - Prefer `Option<T>` over null checks
   - Use `async/await` for I/O operations
   - Leverage Rust's type system for safety

2. **Error Handling:**
   - Use custom error types
   - Return proper HTTP status codes
   - Include error messages in responses
   - Log errors with context
   - Never panic in production code

3. **Database Queries:**
   - Use SQLx compile-time checked queries
   - Use connection pooling
   - Handle database errors gracefully
   - Use transactions for multi-step operations

4. **API Design:**
   - RESTful endpoints with proper HTTP methods
   - Consistent JSON response format
   - Input validation
   - Rate limiting on sensitive endpoints
   - CORS configuration

5. **Performance:**
   - Use connection pooling
   - Implement caching (Redis)
   - Use async I/O (Axum, SQLx)
   - Optimize database queries
   - Use streaming for large responses

### Service Implementation Pattern

**Axum Service Pattern:**
```rust
use axum::{
    routing::{get, post},
    Router, Json,
};
use sqlx::PgPool;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
    })
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/users", get(list_users))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
```

---

## Critical Implementation Requirements

### Current Status

**Infrastructure:**
- ✅ Docker Compose configuration
- ✅ Database schema initialized (10 tables)
- ✅ Network configuration (llm_network)
- ✅ Traefik routing with Authelia
- ✅ Cloudflare Tunnel integration
- ✅ Health check endpoints

**Services:**
- ✅ User Service (stub implementation)
- ✅ Content Service (stub implementation)
- ✅ Storage Service (stub implementation)
- ✅ Editor Service (stub implementation)
- ⚠️ Messaging Service (not deployed)
- ⚠️ Discovery Service (not deployed)
- ⚠️ Subscription Service (not deployed)
- ⚠️ Audio/Video/Graphics Services (not deployed)

**Frontend:**
- ✅ Leptos WASM application (basic shell)
- ⚠️ No login form (relies on Authelia redirect)
- ⚠️ No dashboard UI
- ⚠️ No story creation tools
- ⚠️ No editor integration

### Implementation Priorities

**Phase 1: Core Functionality**
1. User Service - JWT authentication, user management
2. Content Service - AI generation with Claude API
3. Storage Service - MinIO integration, file uploads
4. Editor Service - Basic editing functionality

**Phase 2: Advanced Features**
5. Messaging Service - Real-time collaboration
6. Discovery Service - Content search and recommendations
7. Subscription Service - Payment processing

**Phase 3: Media Transformation**
8. Audio Service - Text-to-speech
9. Video Service - Text-to-video
10. Graphics Service - Text-to-graphic novel

---

## Best Practices

### Security

1. **Authentication:**
   - Always validate JWT tokens
   - Use secure password hashing (argon2)
   - Implement rate limiting on auth endpoints
   - Support MFA for sensitive operations

2. **Input Validation:**
   - Validate all user input
   - Sanitize content (XSS prevention)
   - Limit content length
   - Validate file uploads

3. **Database Security:**
   - Use parameterized queries (SQLx)
   - Never concatenate user input into SQL
   - Use connection pooling with limits
   - Rotate database credentials regularly

4. **Secrets Management:**
   - Use environment variables (never hardcode)
   - Rotate secrets regularly
   - Use different secrets per environment
   - Never commit secrets to git

### Performance

1. **Caching:**
   - Cache frequently accessed data (Redis)
   - Use in-memory caches for hot data
   - Implement cache invalidation strategies
   - Cache user sessions and tokens

2. **Database Optimization:**
   - Add indexes on foreign keys and search columns
   - Use connection pooling
   - Optimize queries (EXPLAIN ANALYZE)
   - Use pagination for large result sets

3. **API Optimization:**
   - Implement response compression
   - Use CDN for static assets
   - Optimize images and media
   - Use streaming for large responses

### Monitoring

1. **Logging:**
   - Use structured logging (tracing)
   - Include request IDs in logs
   - Log errors with context
   - Use appropriate log levels

2. **Metrics:**
   - Expose Prometheus metrics
   - Track request rates, latencies, errors
   - Monitor database connection pool
   - Track cache hit rates

3. **Health Checks:**
   - Implement `/health` endpoint
   - Check database connectivity
   - Check Redis connectivity
   - Check external service dependencies

---

## Deployment Procedures

### Homelab Deployment

1. **Prerequisites:**
   ```bash
   export DOMAIN=leopaska.xyz
   export POSTGRES_PASSWORD=postgresstrongpassword123
   export REDIS_PASSWORD=redisstrongpassword123
   export MINIO_ROOT_USER=minioadmin
   export MINIO_ROOT_PASSWORD=minioadmin123
   export JWT_SECRET=$(openssl rand -base64 32)
   ```

2. **Build and Deploy:**
   ```bash
   docker-compose -f docker-compose.homelab.yml build
   docker-compose -f docker-compose.homelab.yml up -d
   ```

3. **Verify Deployment:**
   ```bash
   docker ps --filter name=authorworks
   curl https://authorworks.leopaska.xyz/health
   bash scripts/verify-homelab.sh
   ```

### Kubernetes Deployment

1. **Apply Manifests:**
   ```bash
   kubectl apply -f k8s/
   ```

2. **Check Status:**
   ```bash
   kubectl get pods -n authorworks
   kubectl get services -n authorworks
   kubectl get ingress -n authorworks
   ```

3. **View Logs:**
   ```bash
   kubectl logs -n authorworks -l app=authorworks -f
   ```

---

## Configuration Files Reference

### Key Configuration Files

1. **docker-compose.homelab.yml** - Homelab deployment
2. **docker-compose.yml** - Local development
3. **nginx.conf** - API Gateway configuration
4. **init-schema.sql** - Database schema
5. **Cargo.toml** - Workspace dependencies
6. **spin.toml** - Spin WASM manifest
7. **Makefile** - Build automation
8. **k8s/** - Kubernetes manifests

### Documentation Files

- **README.md** - Project overview
- **PROMPT.md** - Deployment integration guide
- **DEPLOYMENT.md** - Deployment procedures
- **DEPLOYMENT_STATUS.md** - Current deployment state
- **SERVICE_ARCHITECTURE.md** - Service architecture
- **SPIN_DEPLOYMENT.md** - Spin WASM deployment
- **IMPLEMENTATION_PLAN.md** - Implementation roadmap

---

## Troubleshooting

### Common Issues

**Services not starting:**
- Check logs: `docker-compose -f docker-compose.homelab.yml logs -f`
- Verify environment variables are set
- Check database connectivity
- Verify network configuration

**Database connection errors:**
- Test connection: `docker exec 2f74558da076_neon-postgres-leopaska psql -U postgres -d authorworks -c "SELECT 1"`
- Verify DATABASE_URL environment variable
- Check network connectivity

**404 errors:**
- Verify Traefik routing configuration
- Check service health endpoints
- Verify Cloudflare Tunnel configuration

**Authentication issues:**
- Verify Authelia middleware configuration
- Check JWT_SECRET is set
- Verify Authelia user has correct groups

### Debugging Commands

```bash
# Check container status
docker ps --filter name=authorworks

# View logs
docker-compose -f docker-compose.homelab.yml logs -f [service-name]

# Test health endpoints
curl http://10.0.1.40:8080/health
curl http://10.0.1.37:3001/health

# Database access
docker exec 2f74558da076_neon-postgres-leopaska psql -U postgres -d authorworks

# Network connectivity
docker exec authorworks-user-service ping storage-service
```

---

## Future Enhancements

### Planned Features

1. **AI Generation:**
   - Context-aware prompt engineering
   - Character-aware generation
   - Streaming responses
   - Generation history tracking

2. **Collaboration:**
   - Real-time editing
   - Multi-user collaboration
   - Comment system
   - Version control

3. **Media Transformation:**
   - Text-to-speech
   - Text-to-video
   - Text-to-graphic novel
   - Multi-format exports

4. **Discovery:**
   - Vector search
   - Content recommendations
   - Trending content
   - Category browsing

5. **Monetization:**
   - Subscription tiers
   - Payment processing
   - Revenue sharing
   - Analytics

---

## Implementation Guidelines for AI Agents

### When Making Changes

1. **Always:**
   - Check existing code before modifying
   - Follow Rust best practices (rustfmt, clippy)
   - Use existing error types
   - Write tests for new functionality
   - Update documentation if needed
   - Verify changes don't break existing functionality

2. **Never:**
   - Replace existing code with placeholders
   - Make changes if none are required
   - Use unstable Rust features
   - Ignore compiler warnings
   - Skip error handling
   - Hardcode values (use environment variables)

3. **Code Style:**
   - Use `rustfmt` formatting
   - Pass `clippy` with `-D warnings`
   - Use meaningful variable names
   - Add comments for complex logic
   - Follow existing code patterns

4. **Dependencies:**
   - Use latest stable versions
   - Check compatibility with existing dependencies
   - Verify no breaking changes
   - Update Cargo.lock after changes

5. **Testing:**
   - Write unit tests for new functions
   - Add integration tests for new endpoints
   - Test error cases
   - Verify edge cases

6. **Documentation:**
   - Update README if adding features
   - Document API changes
   - Update architecture docs if needed
   - Add code comments for complex logic

### Git Commit Messages

End each message with a lowercase, one-line git commit message without commands or punctuation. If there are no issues, say "it's clean".

Examples:
- "add jwt authentication to user service"
- "fix database connection issue"
- "implement claude api integration"
- "its clean"

---

## Quick Reference

### Service Ports

- API Gateway: 8080
- User Service: 3001
- Content Service: 3002
- Storage Service: 3003
- Editor Service: 3004
- Subscription Service: 3005
- Messaging Service: 3006
- Discovery Service: 3007
- Audio Service: 3008
- Video Service: 3009
- Graphics Service: 3010

### Key URLs

- Production: https://authorworks.leopaska.xyz
- Health: https://authorworks.leopaska.xyz/health
- API Base: https://authorworks.leopaska.xyz/api

### Database

- Host: 2f74558da076_neon-postgres-leopaska
- Port: 5432
- Database: authorworks
- User: postgres
- Password: postgresstrongpassword123

### Network

- Network: llm_network
- Subnet: 10.0.1.0/24
- Gateway: 10.0.1.40

---

**Last Updated:** January 2025
**Status:** Infrastructure Complete - Application Implementation Pending
**Next Steps:** Implement core services (User, Content, Storage, Editor)

