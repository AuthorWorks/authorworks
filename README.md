# AuthorWorks

AI-assisted long-form writing platform. The v1 product ships a Next.js 14
frontend, a PostgreSQL-backed API, the `book-generator` Rust core library,
and Logto for authentication.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Quick start

```bash
git clone https://github.com/authorworks/authorworks.git
cd authorworks

# Configure secrets (required values are documented inline)
cp .env.example .env
${EDITOR:-vi} .env

# Start the full stack (Postgres, Logto, Nginx, Next.js, workers)
./scripts/deploy.sh local --build

# Open the app
open http://localhost:8080
```

The deploy script wraps `docker compose` with `docker-compose.yml` for the
default `local` target. See `./scripts/deploy.sh --help` for the full list of
environments.

## What ships in v1

- **`frontend/app`** – Next.js 14 application. Owns auth (Logto PKCE),
  `/api/*` routes (books, chapters, dashboard, subscription, profile,
  AI generation/enhance), the rich-text editor (Slate/Plate), and the
  marketing/landing pages.
- **`frontend/landing`** – Static landing pages.
- **`core/book-generator`** – Rust library that performs outline → chapter →
  manuscript expansion and EPUB/PDF rendering. Used by the long-running book
  generation worker.
- **`workers/content`** and **`workers/media`** – Rust background workers
  invoked from the Next.js generation routes for async outline → manuscript
  pipelines.
- **PostgreSQL** – Source of truth for users, books, chapters, generation
  jobs, and credit ledger. Migrations live in `scripts/migrations/`.
- **Logto** – OAuth 2.0 / OIDC identity provider. The Next.js app exchanges
  authorization codes for sessions and stores HttpOnly cookies.

The Rust microservices under `services/` (`user`, `content`, `storage`,
`editor`, `subscription`, `messaging`, `discovery`, `media`) are scaffolding
for a future Spin-on-Kubernetes deployment. They compile against the
`spin-sdk` and are **not** part of the v1 hot path; the Next.js app talks to
PostgreSQL directly.

## Architecture

```
┌────────────────────┐      ┌───────────────────────────────────┐
│  Browser / Reader  │ ───▶ │            Nginx (8080)           │
└────────────────────┘      └────────────────┬──────────────────┘
                                             │
                                             ▼
                            ┌────────────────────────────────────┐
                            │  Next.js 14  (frontend/app)        │
                            │  • PKCE auth flow with Logto       │
                            │  • REST routes under /api/*        │
                            │  • Rich-text editor (Slate/Plate)  │
                            └────┬───────────────┬───────────────┘
                                 │               │
                                 ▼               ▼
                       ┌─────────────────┐  ┌────────────────────────┐
                       │   PostgreSQL    │  │  AI Provider           │
                       │  (content schema│  │  (Anthropic / Ollama)  │
                       │   or fallback)  │  │  via lib/ai.ts         │
                       └────────┬────────┘  └────────────────────────┘
                                ▲
                                │ async generation jobs
                                │
                       ┌────────┴────────────────────────┐
                       │  workers/content + book-gen     │
                       │  (Rust, container-based)        │
                       └─────────────────────────────────┘
```

## Tech stack

| Component | Technology |
|-----------|------------|
| Frontend | Next.js 14, React 18, TanStack Query, Slate/Plate, Tailwind CSS |
| API | Next.js Route Handlers (Node.js runtime), `pg` for PostgreSQL |
| Auth | Logto (OIDC + PKCE), HttpOnly session cookies |
| AI | Anthropic Messages API or Ollama (configurable in `lib/ai.ts`) |
| Long-form generation | Rust workers + `core/book-generator` (EPUB/PDF) |
| Database | PostgreSQL 16 |
| Reverse proxy | Nginx |
| Deployment | Docker Compose (local/EC2), Kubernetes manifests under `k8s/` |

## Repository layout

```
authorworks/
├── frontend/
│   ├── app/                Next.js 14 app (the v1 product)
│   └── landing/            Static landing pages
├── core/book-generator/    Rust crate: outline → manuscript pipeline
├── workers/
│   ├── content/            Async generation worker
│   └── media/              Cover/asset processing worker
├── services/               Future Spin microservices (not in v1 hot path)
├── server/, server-wrapper/Reference Rust HTTP gateways
├── k8s/                    Kubernetes manifests + Argo CD apps
├── terraform/              AWS infra (EKS, EC2)
├── config/                 Service config (Nginx, Logto, Prometheus…)
├── scripts/                Deployment helpers + SQL migrations
└── docs/                   Operator-facing documentation
```

## Development

```bash
# Frontend only (assumes Postgres + Logto running elsewhere)
cd frontend/app
npm install
npm run dev          # http://localhost:3000

# Full stack via Docker Compose
./scripts/deploy.sh local --build       # bring up
./scripts/deploy.sh local --logs        # tail logs
./scripts/deploy.sh local --verify      # health checks
./scripts/deploy.sh local --down        # tear down

# Rust workspace
cargo check --workspace
cargo test  --workspace
```

## Configuration

All runtime configuration is via environment variables. Copy
[`.env.example`](./.env.example) to `.env` and fill in the values marked
`REQUIRED`. The Next.js app additionally reads `frontend/app/.env.local`
during local development; the deploy script forwards values from the root
`.env`.

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | PostgreSQL connection string used by the Next.js API |
| `LOGTO_ENDPOINT` / `LOGTO_APP_ID` / `LOGTO_APP_SECRET` | Logto credentials |
| `LOGTO_REDIRECT_URI` | Callback URL registered in Logto |
| `NEXT_PUBLIC_APP_URL` | Public URL of the Next.js app |
| `AI_PROVIDER` (`anthropic` \| `ollama`) | Default AI backend |
| `ANTHROPIC_API_KEY`, `OLLAMA_URL`, `OLLAMA_MODEL` | Provider-specific |
| `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET` | Optional billing |
| `SUBSCRIPTION_SERVICE_URL` | Optional external billing proxy |

## Deployment

Local + EC2 use Docker Compose. The homelab cluster (k3s) is fully GitOps
via ArgoCD + Image Updater: CI publishes `sha-<commit>` images to GHCR,
Image Updater bumps the kustomize image override on the live Application,
ArgoCD redeploys. No manual `kubectl rollout restart`.

**Homelab quick path:**

1. Verify cluster prereqs. Most are already in place; the only one usually outstanding on a fresh cluster is the `argocd/ghcr-credentials` Secret (a dockerconfigjson with a GHCR PAT). Full list, rationale, and current status in [`docs/CLUSTER_PREREQS.md`](./docs/CLUSTER_PREREQS.md) (also inlined as [Section 0 of the homelab checklist](./docs/HOMELAB_SETUP_CHECKLIST.md#0-cluster-prerequisites)).
2. `./scripts/seal-secrets.sh` — re-seals `authorworks-secrets` and `ghcr-pull-secret` against the current cluster's controller key. Commit and push the resulting files.
3. `./scripts/bootstrap-argocd.sh` — pre-flights, applies `k8s/argocd/app-of-apps.yaml`, waits for `Application/authorworks-homelab` to be Healthy.

Docs:

- [`docs/deployment.md`](./docs/deployment.md) — all environments (local, homelab, EC2, EKS)
- [`docs/HOMELAB_SETUP_CHECKLIST.md`](./docs/HOMELAB_SETUP_CHECKLIST.md) — first-time homelab setup, prereqs included inline
- [`docs/CLUSTER_PREREQS.md`](./docs/CLUSTER_PREREQS.md) — standalone handoff for the engineer who maintains the cluster

## Documentation

- [Getting started](./docs/getting-started.md)
- [Architecture](./docs/architecture.md)
- [Deployment](./docs/deployment.md)
- [Authentication](./docs/authentication.md)

## Contributing

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/...`).
3. Run `npm run lint` in `frontend/app` and `cargo check --workspace`.
4. Open a pull request.

## License

MIT License — see [LICENSE](LICENSE).


<!-- homelab-deployment:begin -->
## Homelab Deployment

This service is deployed to production on the [`l3ocifer/homelab`](https://github.com/l3ocifer/homelab) K3s cluster via ArgoCD GitOps. **AuthorWorks is self-managed** — the entry in the shared [`production-apps`](https://github.com/l3ocifer/homelab/blob/main/argocd/apps/production-apps.yaml) ApplicationSet was removed on 2026-04-28 because it could only track one image annotation; this repo now carries its own `k8s/argocd/applicationset.yaml` with per-image annotations for `frontend`, `book-generator`, and `content-worker`.

### Cluster footprint

| Field | Value |
|---|---|
| **ArgoCD Application** | `authorworks-homelab` (created by the in-repo `applicationset.yaml`) |
| **AppSet entry** | [`k8s/argocd/applicationset.yaml`](k8s/argocd/applicationset.yaml) (this repo) |
| **Namespace** | `authorworks` |
| **Public URL** | https://authorworks.leopaska.xyz |
| **Manifest path (this repo)** | `k8s/overlays/homelab/` |
| **Tracked branch** | `main` |
| **Container images** | `ghcr.io/authorworks/frontend:latest`, `ghcr.io/authorworks/book-generator:latest`, `ghcr.io/authorworks/content-worker:latest` |
| **Image auto-update** | ArgoCD Image Updater (newest-build strategy) |

### Required platform resources (provided by homelab)

| Resource | Endpoint / Reference |
|---|---|
| **Postgres** | `homelab-pg-rw.databases.svc.cluster.local:5432` (CloudNativePG; DB created via `postInitSQL` in [`argocd/apps/_postgres/homelab-pg.yaml`](https://github.com/l3ocifer/homelab/blob/main/argocd/apps/_postgres/homelab-pg.yaml)) |
| **Logto SSO** | `https://logto.leopaska.xyz` |
| **Inference (LiteLLM)** | `http://litellm.inference.svc.cluster.local:4000` (anthropic / openai / vLLM routing) |
| **TLS / DNS** | Cloudflare Tunnel + Traefik IngressRoute, cert via cert-manager (Let's Encrypt DNS-01) |

### SealedSecrets (committed to homelab repo)

| Secret | Type | Keys | Vaultwarden item |
|---|---|---|---|
| `authorworks/authorworks-secrets` | Opaque | `DATABASE_URL`, `LOGTO_APP_ID`, `LOGTO_APP_SECRET`, `ANTHROPIC_API_KEY`, etc. | `authorworks-bundle` |
| `authorworks/ghcr-pull-secret` | dockerconfigjson | `.dockerconfigjson` | `ghcr-leopaska-pat` |

### Re-seal procedure

If a secret reports `no key could decrypt secret (...)` after a cluster rebuild:

1. Plaintext source of truth: self-hosted Vaultwarden at https://warden.leopaska.xyz
2. Follow the step-by-step re-seal flow: [`docs/argocd-triage.md#re-seal-procedure-per-secret`](https://github.com/l3ocifer/homelab/blob/main/docs/argocd-triage.md#re-seal-procedure-per-secret)
3. For the GHCR pull secret use the bulk loop in [`docs/argocd-triage.md#ghcr-pull-secrets-special-case--dockerconfigjson`](https://github.com/l3ocifer/homelab/blob/main/docs/argocd-triage.md#ghcr-pull-secrets-special-case--dockerconfigjson)

### Image build & deploy flow

1. Push to `main` triggers GitHub Actions builds (one workflow per image)
2. Images pushed to GHCR
3. ArgoCD Image Updater detects new digests within ~2m
4. ArgoCD applies updated manifests; rolling restart per Deployment

### Operational references

- **Live status**: https://argocd.leopaska.xyz/applications/authorworks-homelab
- **Logs**: https://grafana.leopaska.xyz → Explore → Loki → `{namespace="authorworks"}`
- **Production apps catalog**: [`docs/production-apps.md`](https://github.com/l3ocifer/homelab/blob/main/docs/production-apps.md)
- **Disaster recovery runbook**: [`docs/disaster-recovery.md`](https://github.com/l3ocifer/homelab/blob/main/docs/disaster-recovery.md)
- **Secrets workflow**: [`docs/secrets-checklist.md`](https://github.com/l3ocifer/homelab/blob/main/docs/secrets-checklist.md)
- **Health triage**: [`docs/argocd-triage.md`](https://github.com/l3ocifer/homelab/blob/main/docs/argocd-triage.md)
<!-- homelab-deployment:end -->
