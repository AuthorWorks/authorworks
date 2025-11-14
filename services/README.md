# AuthorWorks Services

This directory contains all backend microservices for the AuthorWorks platform.

## Services

### Core Services

- **user/** (Port 3001) - User authentication, JWT tokens, profiles, RBAC
- **content/** (Port 3002) - Story/book/chapter management, AI generation orchestration
- **storage/** (Port 3003) - File storage, S3/MinIO integration, exports
- **editor/** (Port 3004) - Collaborative editing, real-time sync, version control

### Supporting Services

- **messaging/** (Port 3006) - WebSocket, event bus, notifications
- **subscription/** (Port 3005) - Stripe integration, billing, payments
- **discovery/** (Port 3007) - Search, recommendations, vector embeddings

### Media Services

- **media/** (Ports 3008-3010) - Consolidated media transformation service
  - Text-to-speech (audio)
  - Text-to-video
  - Text-to-graphic novel (image generation)

## Architecture

All services use:
- **Fermyon Spin** 2.2.0 for serverless WebAssembly deployment
- **Rust** with async/await
- Shared **book-generator** library (../core/book-generator)

## Development

Build all services:
```bash
cd services
for dir in */; do
  cd "$dir"
  spin build
  cd ..
done
```

## History

These services consolidate code from 15 separate submodule repositories:
- 11 empty placeholder repos (deleted)
- 3 repos with duplicated book generator code (consolidated to ../core)
- 1 legacy landing page repo (moved to ../frontend)

This consolidation eliminated ~26,000 LOC of duplication (94% reduction).
