# Deployment Ready Status

**Date:** November 2025
**Status:** ✅ Ready for Deployment

## Changes Made

All deployment configurations have been updated to work with the consolidated monorepo structure.

### Updated Files

1. **docker-compose.homelab.yml**
   - ✅ Fixed all service build context paths (`./services/*` instead of `./authorworks-*-service`)
   - ✅ Updated UI shell volume mount (`./frontend/landing/leptos-app/dist`)
   - ✅ Added all 8 services (user, content, storage, editor, messaging, subscription, discovery, media)
   - ✅ Added environment variables for all services

2. **Dockerfiles (8 new files)**
   - ✅ Created `services/*/Dockerfile` for each service
   - ✅ Multi-stage builds with Rust + WASM
   - ✅ Uses Fermyon Spin runtime

3. **spin.toml (8 new files)**
   - ✅ Created `services/*/spin.toml` for each service
   - ✅ Configured HTTP triggers and build commands

4. **nginx.conf**
   - ✅ Added upstream definitions for new services
   - ✅ Added proxy locations for subscription, messaging, discovery, media APIs
   - ✅ WebSocket support for messaging service

## Deployment Instructions

### For K3D Cluster on Homelab (Aleph)

#### Prerequisites

SSH into your homelab server:
```bash
# Option 1: Direct SSH
ssh alef  # (192.168.1.200:2222)

# Option 2: Via Cloudflare Tunnel
ssh homelab  # (ssh.leopaska.xyz)
```

#### Set Environment Variables

```bash
export DOMAIN=leopaska.xyz
export POSTGRES_PASSWORD=postgresstrongpassword123
export REDIS_PASSWORD=redisstrongpassword123
export MINIO_ROOT_USER=minioadmin
export MINIO_ROOT_PASSWORD=minioadmin123
export JWT_SECRET=$(openssl rand -base64 32)
export ANTHROPIC_API_KEY=your_api_key_here
```

#### Deploy with Docker Compose

```bash
# Navigate to project directory
cd /path/to/authorworks

# Build all services
docker compose -f docker-compose.homelab.yml build

# Deploy
docker compose -f docker-compose.homelab.yml up -d

# Check status
docker compose -f docker-compose.homelab.yml ps

# View logs
docker compose -f docker-compose.homelab.yml logs -f
```

#### Verify Deployment

```bash
# Check health endpoints
curl https://authorworks.leopaska.xyz/health
curl http://localhost:3001/health  # user service
curl http://localhost:3002/health  # content service
curl http://localhost:3003/health  # storage service
curl http://localhost:3004/health  # editor service
curl http://localhost:3005/health  # subscription service
curl http://localhost:3006/health  # messaging service
curl http://localhost:3007/health  # discovery service
curl http://localhost:3008/health  # media service

# Or use the verification script
bash scripts/verify-homelab.sh
```

## Infrastructure Available

Your homelab already has:
- ✅ K3D cluster running
- ✅ PostgreSQL 16 (`neon-postgres-leopaska:5432`)
- ✅ Redis 7 (`redis-nd-leopaska:6379`)
- ✅ MinIO (`minio-leopaska:9000`)
- ✅ Traefik reverse proxy
- ✅ Authelia SSO
- ✅ Cloudflare Tunnel to `*.leopaska.xyz`
- ✅ External network: `llm_network` (10.0.1.0/24)

## Service Ports

| Service | Port | URL |
|---------|------|-----|
| API Gateway | 8080 | https://authorworks.leopaska.xyz |
| User | 3001 | Internal only |
| Content | 3002 | Internal only |
| Storage | 3003 | Internal only |
| Editor | 3004 | Internal only |
| Subscription | 3005 | Internal only |
| Messaging | 3006 | Internal only |
| Discovery | 3007 | Internal only |
| Media | 3008 | Internal only |

## API Endpoints Available

Once deployed, these endpoints will be accessible:

### Authentication
- `POST /api/auth/register`
- `POST /api/auth/login`
- `GET /api/auth/profile`

### Users
- `GET /api/users/`
- `PUT /api/users/profile`

### Content
- `GET /api/content/`
- `POST /api/content/`
- `GET /api/stories/`
- `POST /api/stories/`

### Storage
- `POST /api/upload/`
- `GET /api/storage/`

### Editor
- `POST /api/editor/sessions`
- `GET /api/editor/sessions/{id}`

### Subscriptions
- `POST /api/subscriptions/`
- `GET /api/billing/`

### Messaging
- `WS /api/messaging/` (WebSocket)

### Discovery
- `POST /api/discovery/search`
- `GET /api/discovery/recommendations`

### Media
- `POST /api/media/` (audio, video, graphics)

## Current Service Status

All services are currently **stub implementations** (20-50 LOC each) that return:
- ✅ `/health` endpoint working
- ✅ Basic service info at `/`
- ⚠️ Actual business logic needs implementation

The services will deploy and run, but only provide placeholder responses until the business logic is implemented using the `core/book-generator` library.

## Next Steps for Full Functionality

1. **Test Deploy** - Deploy the stubs to verify infrastructure works
2. **Implement Services** - Add business logic to each service using `core/book-generator`
3. **Integration Testing** - Test service-to-service communication
4. **Frontend Integration** - Connect UI to backend APIs
5. **Production Hardening** - Add monitoring, logging, rate limiting

## Troubleshooting

### Services Won't Build
- Ensure Rust 1.83+ installed: `rustup update`
- Ensure wasm32-wasi target: `rustup target add wasm32-wasi`
- Check Cargo.toml workspace is correct

### Services Won't Start
- Check logs: `docker compose -f docker-compose.homelab.yml logs <service-name>`
- Verify database connectivity: `docker exec <container> curl -f <DATABASE_URL>`
- Check network: `docker network inspect llm_network`

### Can't Access via Domain
- Verify Traefik is running
- Check Cloudflare Tunnel status
- Verify DNS records point to homelab

## SSH Access to Homelab

Your SSH config has two entries:

**alef** - Direct SSH (on local network):
- Host: 192.168.1.200
- Port: 2222
- User: l3o

**homelab** - Via Cloudflare (from anywhere):
- Host: ssh.leopaska.xyz
- Via Cloudflare Access tunnel
- User: l3o

Both use the same identity file: `~/.ssh/leo-personal`

---

**Status:** ✅ All deployment configs updated and ready
**Tested:** Configs validated, services ready to build
**Ready to Deploy:** Yes - Run docker compose up on homelab server
