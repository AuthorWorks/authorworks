AuthorWorks Umbrella Repository

Overview
- Central umbrella repository for the AuthorWorks platform.
- Manages all service and docs repositories as git submodules.
- Contains homelab Docker Compose overlay and gateway config for local/prod MVP.

Quick Start (Homelab)
- export DOMAIN, POSTGRES_PASSWORD, REDIS_PASSWORD, MINIO_ROOT_USER, MINIO_ROOT_PASSWORD
- docker compose -f docker-compose.homelab.yml build
- docker compose -f docker-compose.homelab.yml up -d
- bash scripts/verify-homelab.sh

Key Repositories
- Engine/specs: https://github.com/AuthorWorks/authorworks-engine
- Umbrella (this repo): https://github.com/AuthorWorks/authorworks

