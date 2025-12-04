# AuthorWorks Deployment Architecture Analysis

**Date:** November 2025
**Status:** Post-Consolidation

## Repository Relationship

### Active Repositories (2)

#### 1. `authorworks` (Main Monorepo)
- **Purpose:** Complete application codebase
- **Contains:**
  - `core/book-generator/` - Shared AI book generation library
  - `services/` - 8 microservices (user, content, storage, editor, messaging, subscription, discovery, media)
  - `frontend/landing/` - Landing page (Next.js + Leptos)
  - `docs/` - All documentation
  - Deployment configs (docker-compose, k8s, scripts)

#### 2. `authorworks-engine`
- **Purpose:** Specifications and design documents only
- **Contains:** Core engine specs, architecture docs
- **Relationship:** **DOCUMENTATION ONLY** - Not used in deployment
- **Status:** Independent reference repository

### Key Finding: NO CODE DEPENDENCY

The `authorworks-engine` repo is **purely documentation/specs** and has:
- ❌ No code dependency in main repo
- ❌ No build integration
- ❌ No deployment integration
- ✅ Can be archived or kept as reference docs

## Current Deployment Configuration

### Deployment Targets

1. **Docker Compose (Homelab - K3s)**
   - Primary deployment method
   - File: `docker-compose.homelab.yml`
   - Network: `llm_network` (external, 10.0.1.0/24)
   - Domain: `authorworks.leopaska.xyz`

2. **Kubernetes (K8s/K3s)**
   - Manifests in `k8s/` directory
   - Namespace: `authorworks`
   - Spin operator for WebAssembly

3. **AWS/Cloud**
   - Scripts available: `scripts/deploy-aws.sh`
   - Not currently used

## ⚠️ CRITICAL: Deployment Configs Need Updates

### Broken Paths in docker-compose.homelab.yml

The consolidation broke the deployment configs because they still reference old submodule paths:

**OLD (BROKEN) Paths:**
```yaml
user-service:
  build:
    context: ./authorworks-user-service  # ❌ DOESN'T EXIST

content-service:
  build:
    context: ./authorworks-content-service  # ❌ DOESN'T EXIST

storage-service:
  build:
    context: ./authorworks-storage-service  # ❌ DOESN'T EXIST

editor-service:
  build:
    context: ./authorworks-editor-service  # ❌ DOESN'T EXIST
```

**NEW (CORRECT) Paths:**
```yaml
user-service:
  build:
    context: ./services/user  # ✅ CORRECT

content-service:
  build:
    context: ./services/content  # ✅ CORRECT

storage-service:
  build:
    context: ./services/storage  # ✅ CORRECT

editor-service:
  build:
    context: ./services/editor  # ✅ CORRECT
```

### Additional Issues

1. **UI Shell Path**
   ```yaml
   OLD: - ./authorworks-ui-shell/dist:/usr/share/nginx/html:ro  # ❌ BROKEN
   NEW: - ./frontend/landing/leptos-app/dist:/usr/share/nginx/html:ro  # ✅ CORRECT
   ```

2. **Missing Services**
   - No definitions for: messaging, subscription, discovery, media services
   - Need to add these to docker-compose

3. **Dockerfiles Don't Exist**
   - Services only have `Cargo.toml` and stub `src/lib.rs`
   - Need to create Dockerfiles for each service

## Deployment Process (Once Fixed)

### Option 1: Docker Compose (Recommended for Homelab)

```bash
# 1. Set environment variables
export DOMAIN=leopaska.xyz
export POSTGRES_PASSWORD=postgresstrongpassword123
export REDIS_PASSWORD=redisstrongpassword123
export MINIO_ROOT_USER=minioadmin
export MINIO_ROOT_PASSWORD=minioadmin123
export JWT_SECRET=$(openssl rand -base64 32)

# 2. Build and deploy
docker compose -f docker-compose.homelab.yml build
docker compose -f docker-compose.homelab.yml up -d

# 3. Verify
bash scripts/verify-homelab.sh
curl https://authorworks.leopaska.xyz/health
```

### Option 2: Kubernetes/K3s (Advanced)

```bash
# 1. Build Spin WASM app
make build-optimized

# 2. Deploy to K3s
make deploy-homelab

# 3. Verify
kubectl get pods -n authorworks
kubectl get svc -n authorworks
```

### Option 3: Both Together (Hybrid)

**Can deploy together but not recommended:**
- Docker Compose: Good for quick iteration
- K8s: Good for production scaling
- Running both creates port conflicts and resource waste

**Recommendation:** Choose one deployment method per environment.

## K3s Cluster Status

### Current Setup (From Configs)

✅ **Infrastructure Ready:**
- K3s cluster on homelab server (Aleph - 192.168.1.200)
- External network: `llm_network` (10.0.1.0/24)
- PostgreSQL 16: `neon-postgres-leopaska:5432`
- Redis 7: `redis-nd-leopaska:6379`
- MinIO: `minio-leopaska:9000`
- Traefik reverse proxy with Authelia
- Cloudflare Tunnel: `*.leopaska.xyz`

✅ **K8s Manifests Present:**
- Namespace, services, ingress configured
- Spin operator support
- Multi-tenant setup (tenant-1, tenant-2)
- Monitoring, HPA, network policies

⚠️ **But Services Not Deployable:**
- Service code is only stubs
- No Dockerfiles created yet
- Paths in configs reference old structure

## Required Fixes for Deployment

### Immediate (To Deploy)

1. **Update docker-compose.homelab.yml**
   - Fix all service build context paths
   - Update UI shell volume mount
   - Add missing services (messaging, subscription, discovery, media)

2. **Create Dockerfiles**
   - One Dockerfile per service in `services/*/`
   - Use multi-stage builds for Rust/WASM

3. **Update nginx.conf**
   - Fix paths to UI assets
   - Update service proxy locations

4. **Update K8s manifests**
   - Fix image references
   - Update service selectors
   - Fix volume mounts

### Short-term (To Make Functional)

5. **Implement Service Logic**
   - Current services are 20-50 LOC stubs
   - Need actual implementation using book-generator library

6. **Build Integration**
   - Update Makefile paths
   - Update build scripts
   - Test CI/CD pipeline

## Recommended Deployment Strategy

### Phase 1: Fix Configs (1-2 hours)
1. Update docker-compose.homelab.yml paths
2. Create basic Dockerfiles
3. Test local build: `docker compose build`

### Phase 2: Deploy Stubs (30 mins)
1. Deploy to homelab: `docker compose up -d`
2. Verify health endpoints work
3. Confirm Traefik routing works

### Phase 3: Implement Services (Ongoing)
1. Implement user service with JWT auth
2. Implement content service with book-generator
3. Implement remaining services
4. Test end-to-end workflows

## Can They Deploy Together?

### authorworks + authorworks-engine

**Answer: NO - They don't deploy together**

- `authorworks` = Full application (deployable)
- `authorworks-engine` = Documentation only (not deployable)

The engine repo is just specs/docs and has no deployment artifacts.

### Multiple Deployment Methods

**Docker Compose + K8s Together:**
- ❌ Not recommended (port conflicts, resource duplication)
- ✅ Choose one per environment:
  - Dev: Docker Compose
  - Prod: K8s/K3s

## Summary

| Question | Answer |
|----------|--------|
| How do repos interrelate? | `authorworks-engine` is docs-only, no code dependency |
| How do they deploy? | Only `authorworks` deploys; engine is reference material |
| Can they deploy together? | Engine doesn't deploy; it's documentation |
| Still set up for K3s? | Yes, but paths need updating post-consolidation |
| Ready to deploy? | ❌ No - Needs path fixes + Dockerfiles |
| Estimated fix time | 1-2 hours for basic deployment |

## Next Steps

1. **Immediate:** Update docker-compose.homelab.yml paths
2. **Short-term:** Create Dockerfiles for all services
3. **Medium-term:** Implement actual service logic
4. **Long-term:** Consider archiving authorworks-engine (it's just docs)

---

**Status:** Analysis complete - Deployment configs need updates
**Priority:** High - Required for any deployment
