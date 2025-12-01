AuthorWorks Umbrella Repository

Overview
- Central umbrella repository for the AuthorWorks platform.
- Contains homelab Docker Compose overlay and gateway config for local/prod MVP.
- Manages the authorworks-engine repository as a git submodule (specs and core engine).

Quick Start (Homelab)
- export DOMAIN, POSTGRES_PASSWORD, REDIS_PASSWORD, MINIO_ROOT_USER, MINIO_ROOT_PASSWORD
- docker compose -f docker-compose.homelab.yml build
- docker compose -f docker-compose.homelab.yml up -d
- bash scripts/verify-homelab.sh

Key Repositories
- Engine/specs: https://github.com/AuthorWorks/authorworks-engine (active)
- Umbrella (this repo): https://github.com/AuthorWorks/authorworks (active)

Note: Individual service repositories have been archived. The platform now uses a consolidated architecture with the engine repository containing specifications and core functionality.

