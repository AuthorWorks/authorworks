# AuthorWorks Platform

**AI-powered creative content generation platform** for novels, screenplays, and multimedia content.

## 🎯 Overview

AuthorWorks is a comprehensive platform that combines:
- **AI-Powered Generation** - Context-aware story generation with Anthropic Claude
- **Collaborative Editing** - Real-time editing with version control
- **Multi-Media Transformation** - Convert text to audio, video, graphic novels
- **Microservices Architecture** - Serverless WebAssembly with Fermyon Spin
- **Enterprise Infrastructure** - Docker/Kubernetes deployment ready

## 📁 Repository Structure

```
authorworks/
├── core/
│   └── book-generator/     # Shared AI book generation library (8,785 LOC)
├── services/               # All backend microservices
│   ├── user/              # Authentication & profiles (Port 3001)
│   ├── content/           # Story management (Port 3002)
│   ├── storage/           # S3/MinIO integration (Port 3003)
│   ├── editor/            # Collaborative editing (Port 3004)
│   ├── messaging/         # WebSocket & events (Port 3006)
│   ├── subscription/      # Stripe & billing (Port 3005)
│   ├── discovery/         # Search & recommendations (Port 3007)
│   └── media/             # Audio+Video+Graphics (Ports 3008-3010)
├── frontend/
│   └── landing/           # Marketing website (Next.js/Leptos)
├── docs/                  # Consolidated documentation
├── k8s/                   # Kubernetes manifests
├── scripts/               # Deployment automation
└── archive/               # Legacy submodules (for reference)
```

## 🚀 Quick Start

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Docker
docker --version
```

### Environment Setup
```bash
export DOMAIN=leopaska.xyz
export POSTGRES_PASSWORD=postgresstrongpassword123
export REDIS_PASSWORD=redisstrongpassword123
export MINIO_ROOT_USER=minioadmin
export MINIO_ROOT_PASSWORD=minioadmin123
export JWT_SECRET=$(openssl rand -base64 32)
```

### Build & Run (Local Development)
```bash
# Build all services
cargo build --workspace

# Run with Docker Compose
docker compose -f docker-compose.yml up -d

# Verify deployment
bash scripts/verify-homelab.sh
```

### Build & Run (Homelab Production)
```bash
docker compose -f docker-compose.homelab.yml build
docker compose -f docker-compose.homelab.yml up -d
```

## 🏗️ Architecture

### Technology Stack

**Backend:**
- Rust (edition 2021)
- Fermyon Spin 2.2.0 (Serverless WebAssembly)
- Axum 0.7+ (HTTP framework)
- PostgreSQL 16 + SQLx
- Redis 7 (caching)
- MinIO (S3-compatible storage)

**Frontend:**
- Leptos (Rust/WASM) or Next.js (pending decision)
- Plate.js rich text editor (planned)

**AI/ML:**
- Anthropic Claude API
- Qdrant vector database
- Text-to-speech/video/image generation

**Infrastructure:**
- Docker + Docker Compose
- Kubernetes/K3s
- Traefik reverse proxy
- Authelia SSO
- Cloudflare Tunnel

### Services

| Service | Port | Description |
|---------|------|-------------|
| Gateway | 8080 | Nginx API gateway |
| User | 3001 | Authentication, JWT, profiles |
| Content | 3002 | Story generation & management |
| Storage | 3003 | File storage, S3/MinIO |
| Editor | 3004 | Collaborative editing |
| Subscription | 3005 | Stripe, billing, payments |
| Messaging | 3006 | WebSocket, events |
| Discovery | 3007 | Search, recommendations |
| Media | 3008-3010 | Audio, video, graphics |

## 📚 Documentation

- **[CONSOLIDATION.md](CONSOLIDATION.md)** - Repository consolidation report
- **[PROMPT.md](PROMPT.md)** - Complete system documentation
- **[SERVICE_ARCHITECTURE.md](SERVICE_ARCHITECTURE.md)** - Service architecture
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Deployment procedures
- **[docs/](docs/)** - API standards, error handling, etc.

## 🔧 Development

### Build Services
```bash
# Build all services
cargo build --workspace

# Build specific service
cargo build -p authorworks-user-service

# Build book generator library
cargo build -p book-generator

# Run tests
cargo test --workspace
```

### Service Development
```bash
cd services/user
cargo run

# Watch for changes
cargo watch -x run
```

## 🌐 Deployment

### Production URL
https://authorworks.leopaska.xyz

### Database
- Host: neon-postgres-leopaska
- Port: 5432
- Database: authorworks
- User: postgres

### Network
- Network: llm_network (10.0.1.0/24)
- Gateway IP: 10.0.1.40

## 📊 Recent Changes

### Repository Consolidation (November 2025)
- ✅ Consolidated 15 submodules → 1 monorepo
- ✅ Eliminated ~26,000 LOC of duplication (94% reduction)
- ✅ Extracted shared book-generator library
- ✅ Created unified services structure
- ✅ Removed 11 empty placeholder repositories

See [CONSOLIDATION.md](CONSOLIDATION.md) for details.

## 🤝 Contributing

1. Clone repository: `git clone https://github.com/AuthorWorks/authorworks.git`
2. Create feature branch: `git checkout -b feature-name`
3. Build and test: `cargo build --workspace && cargo test --workspace`
4. Commit changes: `git commit -m "description"`
5. Push and create PR

## 📝 License

See [LICENSE](LICENSE) file.

## 🔗 Links

- **Production**: https://authorworks.leopaska.xyz
- **GitHub**: https://github.com/AuthorWorks/authorworks
- **Documentation**: https://github.com/AuthorWorks/authorworks/tree/main/docs

---

**Status:** Infrastructure complete - Service implementation in progress
**Last Updated:** November 2025
