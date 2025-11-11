# AuthorWorks Full Implementation Plan

## Overview
Build complete, production-ready AuthorWorks platform with actual working features, supporting both Docker and Spin/WASM deployments.

## Deployment Strategy

### Phase 1: Docker Deployment (Current - Immediate Value)
**Status**: Infrastructure ready, need to implement application logic
**Timeline**: Now
**Benefits**:
- Faster development iteration
- Full Rust ecosystem access
- Database and external service integration
- Can deploy immediately on existing Docker infrastructure

### Phase 2: Spin/WASM Deployment (Future - Scalability)
**Status**: Configuration ready, implement after Docker version working
**Timeline**: After Phase 1 complete
**Benefits**:
- Serverless scalability
- Lower resource usage
- WebAssembly sandboxing
- K3s SpinKube integration

## Implementation Approach

Since the infrastructure is already working with Docker Compose, we'll:
1. **Build full features in Docker first** (existing setup)
2. **Test and validate** with real users
3. **Then migrate to Spin/WASM** when needed for scale

This pragmatic approach gets working software to users faster.

## Phase 1: Docker Implementation (Current Focus)

### 1. User Service ✅ Infrastructure Ready
**Current**: Stub returning placeholder JSON
**Implement**:
- [ ] JWT token generation and validation
- [ ] User registration endpoint
- [ ] User login endpoint
- [ ] Password hashing with bcrypt/argon2
- [ ] User profile CRUD operations
- [ ] Integration with Authelia OAuth (read Remote-User header)
- [ ] Subscription tier management

**Files**:
- `authorworks-user-service/src/main.rs` - Entry point
- `authorworks-user-service/src/auth.rs` - JWT and auth logic
- `authorworks-user-service/src/models.rs` - User models
- `authorworks-user-service/src/handlers.rs` - HTTP handlers
- `authorworks-user-service/src/db.rs` - Database operations

**Dependencies to Add**:
```toml
jsonwebtoken = "9"
argon2 = "0.5"
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio", "uuid"] }
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 2. Content Service - AI Generation
**Current**: Stub returning placeholder JSON
**Implement**:
- [ ] Anthropic Claude API integration
- [ ] Context-aware prompt engineering
- [ ] Book/chapter/scene generation
- [ ] Generation history tracking
- [ ] Token usage monitoring
- [ ] Streaming responses
- [ ] Character-aware context building

**Files**:
- `authorworks-content-service/src/main.rs`
- `authorworks-content-service/src/claude.rs` - Claude API client
- `authorworks-content-service/src/prompts.rs` - Prompt templates
- `authorworks-content-service/src/generation.rs` - Generation logic
- `authorworks-content-service/src/context.rs` - Context building

**Dependencies to Add**:
```toml
reqwest = { version = "0.11", features = ["json", "stream"] }
anthropic-sdk = "0.1"  # or direct API calls
futures = "0.3"
tokio-stream = "0.1"
```

### 3. Storage Service - MinIO Integration
**Current**: Stub returning placeholder JSON
**Implement**:
- [ ] MinIO/S3 client integration
- [ ] File upload handling
- [ ] Export generation (PDF, EPUB, DOCX)
- [ ] Version storage
- [ ] Presigned URL generation
- [ ] File metadata tracking

**Files**:
- `authorworks-storage-service/src/main.rs`
- `authorworks-storage-service/src/minio.rs` - S3 client
- `authorworks-storage-service/src/export.rs` - Export formats
- `authorworks-storage-service/src/upload.rs` - Upload handling

**Dependencies to Add**:
```toml
aws-sdk-s3 = "1"
aws-config = "1"
multipart = "0.18"
pdf-writer = "0.8"
```

### 4. Editor Service - Rich Text Editing
**Current**: Stub returning placeholder JSON
**Implement**:
- [ ] Content diff/merge operations
- [ ] Version control logic
- [ ] Collaborative editing coordination
- [ ] Change tracking
- [ ] Conflict resolution

**Files**:
- `authorworks-editor-service/src/main.rs`
- `authorworks-editor-service/src/diff.rs` - Diff engine
- `authorworks-editor-service/src/merge.rs` - Merge logic
- `authorworks-editor-service/src/version.rs` - Versioning

**Dependencies to Add**:
```toml
similar = "2"  # Text diffing
operational-transform = "0.7"
```

### 5. Frontend - Leptos Application
**Current**: Basic Cyberpunk shell
**Implement**:
- [ ] Login/Signup forms (OAuth redirect handling)
- [ ] Dashboard with project cards
- [ ] Story creation wizard
- [ ] Rich text editor (Plate.js or Tiptap integration)
- [ ] AI generation interface
- [ ] Character management UI
- [ ] Export options UI
- [ ] Settings and profile pages

**Files**:
- `authorworks-ui-shell/src/main.rs` - Leptos app entry
- `authorworks-ui-shell/src/components/` - UI components
- `authorworks-ui-shell/src/pages/` - Page components
- `authorworks-ui-shell/src/api/` - API client
- `authorworks-ui-shell/src/state/` - Global state management

**Dependencies to Add** (Leptos project):
```toml
leptos = "0.6"
leptos_router = "0.6"
leptos_meta = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
gloo-net = "0.5"  # HTTP client
wasm-bindgen = "0.2"
```

## Development Priority Order

### Week 1: Authentication & User Management
1. Update User Service Cargo.toml with all dependencies
2. Implement JWT generation and validation
3. Implement login/register endpoints
4. Add database operations with sqlx
5. Test with Authelia integration

### Week 2: Content Generation Core
1. Update Content Service dependencies
2. Integrate Anthropic Claude API
3. Implement basic story generation
4. Add context building from book/character data
5. Test end-to-end generation

### Week 3: Storage & Export
1. Update Storage Service dependencies
2. Integrate MinIO client
3. Implement file upload
4. Add basic export (PDF, DOCX)
5. Test storage operations

### Week 4: Frontend Dashboard
1. Create Leptos project structure
2. Build login/OAuth flow
3. Create dashboard with project cards
4. Add story creation form
5. Integrate with backend APIs

### Week 5: Editor Integration
1. Implement Editor Service logic
2. Integrate rich text editor (Plate.js via WASM bindings or native Rust)
3. Add version control
4. Test collaborative features

### Week 6: Polish & Production
1. Error handling and logging
2. Rate limiting
3. Performance optimization
4. Security audit
5. Production deployment

## Migration to Spin/WASM (Phase 2)

After Docker version is stable and tested:

1. **Create WASM-compatible versions** of each service
   - Use `spin-sdk` instead of `axum`
   - Replace blocking I/O with WASI-compatible alternatives
   - Handle PostgreSQL via Spin's SQL interface
   - Use Spin's key-value store for caching

2. **Update Cargo.toml** for dual builds:
```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# Docker mode
axum = { version = "0.7", optional = true }
tokio = { version = "1", features = ["full"], optional = true }

# Spin mode
spin-sdk = { version = "2.0", optional = true }
```

3. **Build pipeline**:
   - Docker: `cargo build --release --features docker`
   - Spin: `cargo build --target wasm32-wasip1 --release --features spin`

4. **Deploy to K3s** with SpinKube operator

## Immediate Next Steps

1. **Update Cargo.toml files** for all services with full dependencies
2. **Implement User Service** completely (authentication, database operations)
3. **Create basic frontend** with login and dashboard
4. **Integrate Claude API** for content generation
5. **Test end-to-end** user flow

This gets us to a **working product quickly** using the existing Docker infrastructure, then we can optimize with Spin/WASM when we need the scalability benefits.

## Files to Create

- [ ] Implementation for each service (auth, content, storage, editor)
- [ ] Leptos frontend application
- [ ] API client libraries
- [ ] Database migration files
- [ ] Configuration management
- [ ] Testing suite
- [ ] Deployment scripts

## Success Criteria

✅ User can register/login through Authelia
✅ User sees dashboard with their projects
✅ User can create a new story project
✅ User can generate content using AI
✅ User can edit content in rich text editor
✅ User can export to PDF/EPUB
✅ Changes are persisted to PostgreSQL
✅ Files are stored in MinIO
✅ Platform is accessible at https://authorworks.leopaska.xyz

Let's build this! 🚀
