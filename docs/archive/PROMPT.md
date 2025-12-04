# AuthorWorks Platform Deployment and Integration

## Mission

Deploy and operationalize the AuthorWorks AI-assisted content creation platform on the Aleph homelab infrastructure, ensuring seamless integration with existing services, proper authentication via Authelia, and full operational capability accessible at `authorworks.leopaska.xyz`.

## Platform Understanding

AuthorWorks is an AI-powered creative content generation platform that enables users to:
- Generate high-quality long-form creative content (novels, screenplays, plays) using context-aware AI generation
- Edit and refine content with integrated rich text editors
- Publish and distribute to a creator community
- Transform content into multiple media formats (graphic novels, audio, video)
- Monetize through industry connections and subscription support

The platform follows a microservices architecture with the following services:
- **User Service**: Authentication, profiles, subscriptions (JWT-based)
- **Content Service**: Context-aware story generation using Anthropic Claude API
- **Storage Service**: Book storage, versioning, exports to S3/MinIO
- **Editor Service**: Plate.js integration, WYSIWYG editing capabilities
- **Messaging Service**: Real-time collaboration via Matrix/WebSocket
- **Discovery Service**: Content discovery, search, recommendations using vector embeddings
- **Audio Service**: Text-to-audio transformation
- **Video Service**: Text-to-video transformation
- **Graphics Service**: Text-to-graphic novel transformation
- **Subscription Service**: Payment processing via Stripe

## Infrastructure Context

### Existing Homelab Architecture
- **Server**: Aleph (Ubuntu 24.04.2 LTS, kernel 6.14.0-32-generic)
- **Orchestration**: K3s v1.33.4 (single-node control plane)
- **Container Runtime**: containerd 2.0.5-k3s2
- **Networking**: External bridge network `llm_network` (10.0.1.0/24)
- **Ingress**: Traefik with Let's Encrypt certificates
- **Tunnel**: Cloudflare Tunnel routing to `leopaska.xyz` domain
- **Authentication**: Authelia centralized SSO (authelia-leopaska container)
- **Monitoring**: Prometheus, Grafana, Loki stack
- **Registry**: Local Docker registry on port 5000

### Shared Infrastructure Services
- **PostgreSQL**: neon-postgres-leopaska (PostgreSQL 16-alpine)
- **Redis**: redis-nd-leopaska (with password authentication)
- **MinIO**: minio-leopaska (S3-compatible object storage)
- **Message Queue**: Available for async task processing
- **Vector Database**: qdrant for embeddings (discovery service)

### Network Configuration
All production applications connect to the `llm_network` and are exposed via Traefik with:
- Automatic TLS certificate generation
- Authelia middleware for authentication
- Host-based routing (subdomains of leopaska.xyz)

## Current State Assessment

### What Exists
1. **Codebase Structure**: Microservices in workspace with both:
   - WASM/Spin components (`src/lib.rs` - Spin SDK based)
   - Legacy HTTP server code (`backup/main-rs-files/` - Axum-based)
   - Book generator library (`legacy-src/` - functional Anthropic-based generation engine)

2. **Deployment Configurations**:
   - `docker-compose.homelab.yml` - Docker Compose with Traefik labels
   - `k8s/` directory - Kubernetes manifests for Spin/WASM deployment
   - `spin.toml` - Spin application manifest

3. **Infrastructure State**:
   - K3s cluster operational
   - Spin operator installed but currently failing
   - SpinApp resources crash-looping (Exit Code 137)
   - Local OCI registry functional
   - Authelia running and configured

### Problems Identified
1. **Architectural Confusion**: Dual codebase (Spin WASM + traditional HTTP) causing conflicts
2. **Build Failures**: Cargo.toml configured for WASM (`cdylib`) while Dockerfiles expect binaries
3. **Spin/WASM Issues**: OCI images using `application/vnd.wasm.content.layer.v1+wasm` media type incompatible with standard Docker tooling
4. **Missing Dependencies**: Book generator library requires langchain-rust, anthropic SDK, mdbook, dotenv, async-openai, ollama-rs, reqwest with specific features
5. **No Running Application**: Site returns 404 at authorworks.leopaska.xyz
6. **Disk Pressure**: Node experienced disk pressure (now resolved - 80% usage, 47GB free)

## Technical Requirements

### Deployment Architecture Decision

**Choose ONE architecture and implement completely:**

**Option A: Docker Compose Deployment (Recommended for Homelab)**
- Build traditional HTTP services using Axum framework
- Deploy via `docker-compose.homelab.yml` on `llm_network`
- Integrate with existing Traefik/Authelia infrastructure
- Use shared PostgreSQL, Redis, MinIO services
- Simpler operations, easier debugging, consistent with other homelab apps

**Option B: Kubernetes Spin/WASM Deployment**
- Fix Spin OCI image packaging (requires proper wasm32-wasip1 builds)
- Ensure containerd-shim-spin executor functions correctly
- Configure RuntimeClass and SpinAppExecutor properly
- Resolve webhook certificate injection for spin-operator
- More complex, bleeding-edge technology, potential compatibility issues

### Service Implementation Requirements

Each microservice must:

1. **HTTP API Server**:
   - Implement using Axum 0.7+ framework
   - Expose health check endpoint at `/health`
   - Implement service-specific routes per API specification
   - Support CORS with configurable allowed origins
   - Structured logging with tracing/tracing-subscriber
   - Graceful shutdown handling

2. **Environment Configuration**:
   - `HOST`: Bind address (default: 0.0.0.0)
   - `SERVICE_PORT`: Service-specific port
   - `DATABASE_URL`: PostgreSQL connection string
   - `REDIS_URL`: Redis connection string (with password)
   - `ALLOWED_ORIGINS`: CORS configuration
   - `ANTHROPIC_API_KEY`: For content generation services
   - `MINIO_ENDPOINT`, `MINIO_ACCESS_KEY`, `MINIO_SECRET_KEY`: For storage service

3. **Dockerfile Standards**:
   - Multi-stage build (rust:1.89-slim builder, debian:bookworm-slim runtime)
   - Use cargo-chef for dependency caching
   - Build release binaries (not WASM)
   - Run as non-root user (UID 10001)
   - Include curl for healthchecks
   - Proper HEALTHCHECK directive
   - Minimal image size

4. **Database Schema**:
   - Create migrations for authorworks database
   - Tables: users, books, chapters, scenes, subscriptions, etc.
   - Proper indexes for performance
   - Foreign key relationships enforced

### Authelia Integration Specification

Configure Traefik labels for each exposed service:

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.<service>.rule=Host(`authorworks.${DOMAIN}`)"
  - "traefik.http.routers.<service>.entrypoints=websecure"
  - "traefik.http.routers.<service>.tls.certresolver=letsencrypt"
  - "traefik.http.services.<service>.loadbalancer.server.port=<port>"
  - "traefik.http.routers.<service>.middlewares=authelia-leopaska@docker"
```

The `authelia-leopaska@docker` middleware:
- Intercepts all requests before reaching the application
- Redirects unauthenticated users to Authelia login
- Passes authentication headers (Remote-User, Remote-Groups, Remote-Email) to backend
- Maintains session state via encrypted cookies

Services must:
- Trust the `Remote-User` header when present (validates via Authelia secret)
- Implement authorization logic based on `Remote-Groups` header
- Not implement their own authentication (delegate to Authelia)

### Nginx API Gateway Configuration

The `nginx.conf` must:
- Listen on port 8080
- Proxy requests to appropriate backend services:
  - `/api/users/*` → user-service:3001
  - `/api/content/*` → content-service:3002
  - `/api/storage/*` → storage-service:3003
  - `/api/editor/*` → editor-service:3004
  - `/api/messaging/*` → messaging-service:3006
  - `/api/discovery/*` → discovery-service:3007
- Preserve authentication headers from Authelia
- Enable request buffering for large uploads
- Configure appropriate timeouts for long-running generation tasks
- Add health check aggregation endpoint

### UI Shell Requirements

The `authorworks-ui-shell`:
- Serves as the main frontend application
- Built with modern JavaScript framework (React/Leptos/Dioxus)
- Located at `/authorworks-ui-shell/dist`
- Must be built/available before Docker deployment
- Should contain:
  - Login page (redirects to Authelia)
  - Dashboard showing user's books
  - Book generation interface
  - Editor interface
  - Settings and profile management

If `dist/` is empty, either:
- Build the UI from source (check package.json/Cargo.toml in authorworks-ui-shell)
- Create minimal placeholder UI with navigation to API docs
- Serve static HTML with links to API endpoints

### Book Generator Integration

The core book generator library in `legacy-src/` requires:

**Dependencies** (add to Cargo.toml):
```toml
langchain-rust = "4.9"
anthropic = "0.2"
async-openai = "0.25"
ollama-rs = "0.2"
mdbook = "0.4"
dotenv = "0.15"
async-trait = "0.1"
once_cell = "1.20"
reqwest = { version = "0.12", features = ["json", "stream"] }
futures-core = "0.3"
md-5 = "0.10"
```

**Functionality**:
- Uses Anthropic Claude API for text generation
- Implements chain-of-thought generation for context maintenance
- Generates: braindump → genre → style → characters → synopsis → outline → chapters → scenes
- Exports to markdown, PDF (via mdbook), and EPUB
- Tracks token usage and costs
- Saves intermediate results for resumability

**HTTP API Wrapper** (if exposing as service):
- `POST /api/generate` - Start book generation job
- `GET /api/books` - List user's books
- `GET /api/books/:id` - Get book details
- `GET /api/books/:id/status` - Check generation progress
- `GET /api/books/:id/download` - Download PDF/EPUB
- WebSocket `/ws/generate` - Stream generation progress

### Cloudflare Tunnel Configuration

Verify `/home/l3o/git/homelab/services/cloudflare-tunnel/config.yml` includes:

```yaml
ingress:
  - hostname: authorworks.leopaska.xyz
    service: http://localhost:80
    originRequest:
      httpHostHeader: authorworks.leopaska.xyz
```

Ensure DNS CNAME record exists:
- Name: `authorworks`
- Target: `<tunnel-id>.cfargotunnel.com`
- Proxied: Yes

### Deployment Steps

1. **Clean Up Kubernetes Spin Deployment**:
   - Delete failing SpinApp resources: `kubectl delete spinapp --all -n authorworks`
   - Remove test/duplicate resources
   - Consolidate to single `authorworks` namespace
   - Keep SpinKube operator for future use but don't rely on it now

2. **Prepare Source Code**:
   - Update all service `Cargo.toml` files to build binary executables (not cdylib)
   - Remove `src/lib.rs` files from services (keeps Spin code out of binary builds)
   - Restore working `main.rs` from backup or create new Axum servers
   - Remove chrono dependency if unused (or add to Cargo.toml if needed)
   - Ensure all services compile successfully locally

3. **Build Docker Images**:
   ```bash
   export DOMAIN=leopaska.xyz
   export POSTGRES_PASSWORD=<from /home/l3o/git/homelab/services/.env>
   export REDIS_PASSWORD=<from /home/l3o/git/homelab/services/.env>
   export MINIO_ROOT_USER=minioadmin
   export MINIO_ROOT_PASSWORD=minioadmin
   
   docker compose -f docker-compose.homelab.yml build
   ```

4. **Initialize Database**:
   - Connect to neon-postgres-leopaska
   - Create `authorworks` database if not exists
   - Run migrations for all services
   - Create initial tables: users, books, chapters, scenes, subscriptions

5. **Deploy Services**:
   ```bash
   docker compose -f docker-compose.homelab.yml up -d
   ```

6. **Verify Deployment**:
   - Check all containers are healthy: `docker ps`
   - Test health endpoints for each service
   - Verify Traefik routing: check Traefik dashboard
   - Test Authelia integration: access site, verify redirect to Authelia
   - Test end-to-end: authenticate → access dashboard → generate book

7. **Update Ingress Configuration**:
   - Ensure Traefik labels are correct in docker-compose.homelab.yml
   - Verify Authelia middleware is applied
   - Test CORS configuration allows UI to call API

8. **UI Shell Deployment**:
   - Check if `authorworks-ui-shell/dist` contains built frontend
   - If empty, build the UI:
     - Check for package.json → `npm install && npm run build`
     - Check for Cargo.toml (Leptos/Dioxus) → `trunk build --release` or `cargo build --release`
   - Serve via nginx container or as static files through API gateway
   - Configure routing to show dashboard, book list, generation interface

### Configuration Files to Update

**File**: `docker-compose.homelab.yml`
- Remove services with no Dockerfile (audio, video, graphics, subscription)
- Update to include only: user-service, content-service, storage-service, editor-service, messaging-service, discovery-service, api-gateway
- Ensure all services reference correct container names for PostgreSQL/Redis/MinIO
- Add Authelia middleware to all Traefik routers
- Set proper environment variables from homelab services .env

**File**: `nginx.conf`
- Configure upstream servers for all backend services
- Implement routing rules for each API path
- Add error handling for service unavailability
- Configure request timeout for long-running generation tasks (300s+)
- Enable access logs for debugging

**File**: `k8s/namespace.yaml`
- Remove tenant namespaces (authorworks-tenant-1, authorworks-tenant-2)
- Keep only single `authorworks` namespace

**File**: `k8s/spinapp.yaml`
- Remove multi-tenant SpinApp definitions
- Keep single authorworks-platform SpinApp for future use
- Document that Spin deployment is experimental/future work

**File**: `k8s/ingress.yaml`
- Update to single namespace
- Ensure ingressClassName is `traefik` (not nginx)
- Add proper TLS configuration
- Remove tenant-specific ingress rules

### Service-Specific Implementation Details

#### User Service (Port 3001)

**Endpoints**:
- `GET /health` - Health check
- `GET /` - Service info
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - User login (generates JWT)
- `GET /api/v1/users` - List users (admin only)
- `GET /api/v1/users/:id` - Get user profile
- `PATCH /api/v1/users/:id` - Update user profile
- `GET /api/v1/users/:id/books` - List user's books

**Database Tables**:
- `users` (id, email, username, password_hash, created_at, updated_at)
- `profiles` (user_id, display_name, bio, avatar_url)
- `subscriptions` (id, user_id, tier, stripe_subscription_id, status, expires_at)

**Authentication Flow**:
- Authelia provides primary authentication
- Service validates `Remote-User` header
- Generates JWT for API access if needed
- Stores session state in Redis

#### Content Service (Port 3002)

**Endpoints**:
- `GET /health` - Health check
- `POST /api/v1/books/generate` - Start book generation (async job)
- `GET /api/v1/books/:id/status` - Check generation status
- `GET /api/v1/books` - List books (filtered by auth user)
- `GET /api/v1/books/:id` - Get book details
- `DELETE /api/v1/books/:id` - Delete book
- `POST /api/v1/books/:id/chapters/:chapter/scenes` - Generate specific scene
- `GET /api/v1/books/:id/export` - Export book (PDF/EPUB)

**Database Tables**:
- `books` (id, user_id, title, status, metadata_json, created_at, updated_at)
- `chapters` (id, book_id, number, title, description, status)
- `scenes` (id, chapter_id, number, title, content, word_count, status)
- `generation_jobs` (id, book_id, status, progress, error_message, started_at, completed_at)

**Implementation**:
- Wrap book_generator library functions in async HTTP handlers
- Use background task queue for long-running generation
- Store generation state in Redis for progress tracking
- Save generated content to PostgreSQL
- Publish progress events via messaging service

**Book Generator Integration**:
- Mount book_generator as library dependency
- Initialize Config with Anthropic API key from env
- Route generation requests to `generate_book_with_dir()`
- Stream progress updates via WebSocket or SSE
- Handle resumption of partial books
- Calculate and store token usage/costs

#### Storage Service (Port 3003)

**Endpoints**:
- `GET /health` - Health check
- `POST /api/v1/files` - Upload file to MinIO
- `GET /api/v1/files/:id` - Get file metadata
- `GET /api/v1/files/:id/download` - Download file
- `DELETE /api/v1/files/:id` - Delete file
- `POST /api/v1/books/:id/export` - Generate and store PDF/EPUB

**Implementation**:
- Connect to MinIO using AWS S3 SDK (rust-s3 or aws-sdk-s3)
- Create buckets: `authorworks-books`, `authorworks-exports`, `authorworks-uploads`
- Implement multipart upload for large files
- Generate signed URLs for downloads (expire after 1 hour)
- Store file metadata in PostgreSQL

#### Editor Service (Port 3004)

**Endpoints**:
- `GET /health` - Health check
- `GET /api/v1/editor/document/:id` - Load document for editing
- `PUT /api/v1/editor/document/:id` - Save document changes
- `POST /api/v1/editor/document/:id/collaborate` - Start collaborative session
- `WS /ws/editor/:id` - WebSocket for real-time collaboration

**Implementation**:
- Integrate with content service for loading/saving
- Implement Operational Transform or CRDT for collaboration
- Use Redis for collaborative session state
- Support markdown and rich text formats

#### Messaging Service (Port 3006)

**Endpoints**:
- `GET /health` - Health check
- `WS /ws` - WebSocket connection for real-time events
- `POST /api/v1/notifications` - Send notification
- `GET /api/v1/notifications` - Get user notifications

**Implementation**:
- WebSocket server for real-time updates
- Redis pub/sub for message distribution
- Notify on: book generation complete, collaboration requests, comments

#### Discovery Service (Port 3007)

**Endpoints**:
- `GET /health` - Health check
- `GET /api/v1/discover/books` - Search/discover books
- `GET /api/v1/discover/trending` - Trending content
- `GET /api/v1/discover/recommendations` - Personalized recommendations
- `POST /api/v1/discover/index` - Index book for search

**Implementation**:
- Connect to Qdrant vector database
- Generate embeddings for book metadata (title, synopsis, genre)
- Implement semantic search
- Ranking algorithm for recommendations

### Infrastructure as Code Standards

All deployments must be:

1. **Declarative**: Complete configuration in YAML/TOML files
2. **Version Controlled**: All manifests committed to git
3. **Reproducible**: `git clone && docker compose up` should work
4. **Documented**: Inline comments explaining configuration decisions
5. **Environment Separated**: Use `.env` files, never hardcode secrets

**Required IaC Files**:
- `docker-compose.homelab.yml` - Complete Docker Compose with all services
- `k8s/*.yaml` - Kubernetes manifests (for future Spin deployment)
- `nginx.conf` - Complete API gateway configuration
- `.env.example` - Template for required environment variables
- `scripts/deploy.sh` - Automated deployment script
- `scripts/verify.sh` - Health check script

### Testing and Verification Protocol

**Pre-Deployment Checks**:
1. Verify all service Cargo.toml files build binaries (not cdylib)
2. Test local build: `cargo build --release` in each service directory
3. Verify Dockerfiles don't reference missing files
4. Check docker-compose.yml syntax: `docker compose config`
5. Ensure environment variables are set

**Post-Deployment Verification**:
1. **Container Health**: `docker ps` - all services "healthy" status
2. **Service Health**: `curl http://<service>:port/health` for each service
3. **Database Connectivity**: Check each service can connect to PostgreSQL
4. **Redis Connectivity**: Verify session storage works
5. **MinIO Connectivity**: Test file upload/download
6. **Traefik Routing**: Check Traefik dashboard shows all routes
7. **TLS Certificates**: Verify Let's Encrypt certs are issued
8. **Authelia Integration**: Test full authentication flow
9. **API Gateway**: Test all routes through nginx
10. **End-to-End**: Generate a test book through the UI

**Test Commands**:
```bash
# Health checks
for port in 3001 3002 3003 3004 3006 3007; do
  curl -f http://localhost:$port/health || echo "Port $port failed"
done

# API Gateway
curl -f http://localhost:8080/health

# External Access (through Cloudflare)
curl -I https://authorworks.leopaska.xyz
curl https://authorworks.leopaska.xyz/health

# Authentication (should redirect to Authelia)
curl -L https://authorworks.leopaska.xyz/api/books

# Database
docker exec neon-postgres-leopaska psql -U postgres -c '\l' | grep authorworks
```

### Book Generation Workflow Implementation

**User Journey**:
1. User accesses `https://authorworks.leopaska.xyz`
2. Redirected to Authelia login (if not authenticated)
3. After login, redirected back to dashboard
4. Clicks "Generate New Book"
5. Enters book title and optional braindump
6. Clicks "Start Generation"
7. System creates generation job
8. Backend calls book_generator library
9. Progress updates streamed to UI via WebSocket
10. Upon completion, book is saved to database and MinIO
11. User can view, edit, or export (PDF/EPUB)

**Implementation**:
- Content service exposes HTTP endpoint wrapping `generate_book()`
- Use async task queue (tokio spawn or dedicated job queue)
- Store partial results in PostgreSQL for resumability
- Stream progress via messaging service WebSocket
- Handle failures gracefully with retry logic

### Critical Environment Variables

Create `/home/l3o/git/production/authorworks/.env`:
```bash
DOMAIN=leopaska.xyz
POSTGRES_PASSWORD=<from homelab services .env>
REDIS_PASSWORD=<from homelab services .env>
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
ANTHROPIC_API_KEY=<user must provide>
JWT_SECRET=<generate random 64-char hex>
```

### Monitoring and Observability

Configure each service to:
- Export Prometheus metrics at `/metrics`
- Log to stdout in JSON format for Loki aggregation
- Include trace IDs for request correlation
- Track key metrics:
  - Request count/latency per endpoint
  - Book generation success/failure rate
  - Token usage and API costs
  - Database query performance
  - Error rates by type

Add to Prometheus scrape config (in homelab prometheus):
```yaml
- job_name: 'authorworks'
  static_configs:
    - targets: ['authorworks-user-service:3001', 'authorworks-content-service:3002']
```

### Security Considerations

1. **Network Isolation**: All services on `llm_network`, not exposed directly
2. **Authentication**: Mandatory Authelia for all web access
3. **Secrets Management**: Use environment variables, never commit secrets
4. **Database Credentials**: Rotate regularly, use strong passwords
5. **API Rate Limiting**: Implement in nginx (limit generation requests)
6. **Input Validation**: Sanitize all user inputs (book titles, content)
7. **CORS Policy**: Restrict to authorworks.leopaska.xyz origin only
8. **File Upload Limits**: Max 10MB per file, validate MIME types
9. **SQL Injection Prevention**: Use parameterized queries only
10. **XSS Prevention**: Sanitize all HTML output

### Performance Optimization

1. **Database**:
   - Index frequently queried columns (user_id, book_id, status)
   - Use connection pooling (SQLx pool size: 20)
   - Implement query result caching in Redis (1 hour TTL)

2. **Book Generation**:
   - Implement queue system to limit concurrent generations (max 3)
   - Cache LLM responses for common prompts
   - Use streaming responses for real-time feedback
   - Implement checkpointing every chapter

3. **Static Assets**:
   - Serve UI shell through nginx with caching headers
   - Compress responses (gzip/brotli)
   - CDN via Cloudflare for static assets

4. **API Gateway**:
   - Enable nginx caching for read-only endpoints
   - Configure request buffering for large payloads
   - Implement circuit breakers for failing services

### Cleanup Tasks

Remove temporary/unused files:
- `Dockerfile.spin-oci` - Experimental, non-functional
- `Dockerfile.server` - Superseded by service Dockerfiles
- `Dockerfile.production` - Superseded by docker-compose
- `docker-compose.homelab.simple.yml` - Incomplete implementation
- `nginx-simple.conf` - Incomplete implementation
- `server/` directory - Superseded by proper implementation
- `server-wrapper/` directory - Not needed
- `authorworks-user-service/Dockerfile.server` - Duplicate

### Documentation Updates

**File**: `README.md`
- Update with actual deployment instructions
- Document all environment variables
- Add API documentation links
- Include troubleshooting section

**File**: `DEPLOYMENT.md`
- Replace generic Kubernetes examples with actual homelab deployment
- Document Authelia integration
- Add Cloudflare tunnel configuration
- Include monitoring setup

**File**: `SERVICE_ARCHITECTURE.md`
- Update with as-built architecture
- Document service dependencies
- Add sequence diagrams for key workflows

### Success Criteria

The deployment is complete and successful when:

1. ✅ All Docker containers running and healthy
2. ✅ `https://authorworks.leopaska.xyz` accessible in browser
3. ✅ Authelia authentication redirects work correctly
4. ✅ User can log in and access dashboard
5. ✅ All API endpoints return valid responses
6. ✅ Can create a test book through the UI
7. ✅ Book generation completes successfully
8. ✅ Generated book can be downloaded as PDF/EPUB
9. ✅ All services properly log to stdout
10. ✅ Prometheus metrics being collected
11. ✅ Zero error logs in steady state
12. ✅ All IaC files committed to git

### Edge Cases and Error Handling

**Handle These Scenarios**:
- Anthropic API key invalid/expired → Return clear error message
- Anthropic API rate limit hit → Queue requests, retry with backoff
- Database connection lost → Reconnect automatically, fail gracefully
- Redis unavailable → Degrade gracefully (no caching, still functional)
- MinIO unavailable → Can't upload/download, but generation continues
- Long-running generation timeout → Checkpoint and resume
- Partial book generation → Support resume from last completed chapter
- Concurrent edits → Implement conflict resolution
- Disk space full → Alert, prevent new generation jobs
- Service restart during generation → Resume from checkpoint

### Advanced Features (Implement After Core Works)

1. **Multi-User Collaboration**: Real-time editing with CRDT
2. **Template Library**: Pre-built book templates and prompts
3. **AI Fine-Tuning**: Train custom models on user's writing style
4. **Marketplace**: Buy/sell generated content
5. **API Access**: Public API with rate limiting and billing
6. **Webhook Support**: Notify external systems on events
7. **Batch Processing**: Generate multiple books from template
8. **A/B Testing**: Compare different prompts/models
9. **Analytics Dashboard**: Usage statistics, popular genres
10. **Integration APIs**: Export to Medium, Wattpad, etc.

### Dependencies and External Services

**Required External APIs**:
- Anthropic Claude API (claude-3-5-sonnet or claude-3-opus)
- Stripe API (for subscription service)
- SendGrid or AWS SES (for email notifications)
- Optional: OpenAI API (fallback), Ollama (local LLM)

**Homelab Service Dependencies**:
- PostgreSQL: Database storage
- Redis: Caching, sessions, job queue
- MinIO: Object storage for books and exports
- Traefik: Reverse proxy and TLS termination
- Authelia: Authentication and authorization
- Prometheus/Grafana: Monitoring
- Loki: Log aggregation
- Cloudflare Tunnel: External access

### Performance Targets

- API Response Time: < 200ms (p95) for non-generation endpoints
- Book Generation Time: 10-30 minutes for full novel (50k-100k words)
- Concurrent Users: Support 100+ simultaneous users
- Database Queries: < 50ms (p95)
- File Upload: Support up to 100MB files
- WebSocket Latency: < 100ms for real-time updates
- Memory Usage: < 512MB per service container
- CPU Usage: < 1 core per service under normal load

### Rollback Strategy

If deployment fails:
1. Stop new containers: `docker compose -f docker-compose.homelab.yml down`
2. Restore K3s Spin deployment (if it was working previously)
3. Check logs: `docker compose logs` and `kubectl logs`
4. Verify database migrations didn't corrupt data
5. Keep data volumes intact (authorworks-books, authorworks-logs)

### Final Checklist

Before marking complete:
- [ ] All Cargo.toml files updated with correct dependencies
- [ ] All Dockerfiles build successfully
- [ ] docker-compose.homelab.yml has correct service definitions
- [ ] nginx.conf routes all API paths correctly
- [ ] Environment variables sourced from homelab services .env
- [ ] Authelia middleware configured on all routes
- [ ] Database initialized with migrations
- [ ] All services start without errors
- [ ] Health checks return 200 OK
- [ ] Traefik dashboard shows all routes
- [ ] https://authorworks.leopaska.xyz loads in browser
- [ ] Authentication flow works end-to-end
- [ ] Can generate a test book successfully
- [ ] Monitoring dashboards show metrics
- [ ] Documentation updated with actual deployment
- [ ] Cleanup temporary files removed
- [ ] Git commit with all changes
- [ ] Verify no K8s pods crash-looping
- [ ] Consolidated to single authorworks namespace

## Key Technical Decisions Required

**Decision 1: Deployment Platform**
- Docker Compose (simpler, proven with existing homelab apps) **[RECOMMENDED]**
- Kubernetes Spin/WASM (cutting edge, currently non-functional)

**Decision 2: Book Generator Exposure**
- Embed as library in content-service (simpler, single deployment)
- Separate CLI tool called via subprocess (isolates failures)
- Separate microservice (more complexity)

**Decision 3: UI Framework**
- Use existing Leptos app in `author_works/leptos-app/` (build and deploy)
- Create new React/Next.js frontend (more common, easier to find developers)
- Minimal HTML/HTMX interface (fastest to deploy)

**Decision 4: Job Queue**
- Redis-based simple queue (adequate for homelab scale)
- Dedicated queue system like BullMQ or Celery (overkill for now)
- Direct async/await (risk of lost jobs on restart)

## Implementation Principles

1. **Prioritize Working Over Perfect**: Get basic functionality deployed first
2. **Iterate Incrementally**: Start with core features, add complexity later
3. **Fail Fast**: Validate early, return clear error messages
4. **Log Everything**: Comprehensive logging for debugging
5. **Monitor Proactively**: Metrics and alerts before problems occur
6. **Document Decisions**: Explain why, not just what
7. **Secure by Default**: Authentication required, principle of least privilege
8. **Performance Matters**: Optimize critical paths (generation workflow)
9. **User Experience**: Clear feedback, no silent failures
10. **Maintainability**: Code should be readable and well-structured

## Specific Technical Constraints

- **Rust Version**: 1.89+ (latest stable as of Oct 2025)
- **Axum Version**: 0.7.x (latest stable)
- **PostgreSQL**: 16+ (use neon-postgres-leopaska container)
- **Redis**: Compatible with redis-nd-leopaska (v7+)
- **MinIO**: Compatible with minio-leopaska (latest)
- **Docker Compose**: v2.38.2
- **Kubernetes**: v1.33.4+k3s1
- **Traefik**: Latest (already deployed in homelab)
- **Authelia**: Latest (already deployed in homelab)

## Expected Deliverables

1. **Working Deployment**:
   - All services running and accessible
   - AuthorWorks platform functional at authorworks.leopaska.xyz
   - Full authentication via Authelia
   - Can generate books end-to-end

2. **Infrastructure as Code**:
   - Complete `docker-compose.homelab.yml`
   - Updated Kubernetes manifests (single namespace)
   - All configuration files (nginx.conf, .env.example)
   - Deployment scripts

3. **Documentation**:
   - Updated README.md with deployment instructions
   - API documentation for all endpoints
   - Architecture diagram (as-built)
   - Troubleshooting guide

4. **Code Quality**:
   - All services compile without warnings
   - Proper error handling throughout
   - Consistent code style (rustfmt)
   - No hardcoded secrets or credentials

5. **Operational Readiness**:
   - Health checks on all services
   - Logging to stdout (captured by Loki)
   - Prometheus metrics exposed
   - Graceful degradation when dependencies unavailable

This prompt provides the complete context and requirements for deploying AuthorWorks to production on the Aleph homelab infrastructure.
