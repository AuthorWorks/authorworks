# AuthorWorks Documentation

Welcome to the AuthorWorks platform documentation.

## Quick Links

| Document | Description |
|----------|-------------|
| [Getting Started](./getting-started.md) | Quick start guide for developers |
| [Architecture](./architecture.md) | System architecture and design |
| [Deployment](./deployment.md) | Deployment guides for all environments |
| [Homelab Setup Checklist](./HOMELAB_SETUP_CHECKLIST.md) | K3s migrations, apply, content worker, monitoring |
| [Homelab Monitoring](./HOMELAB_MONITORING.md) | Prometheus, Loki, Uptime Kuma, Grafana integration |
| [Agent Logging & Analytics](./agent-logging-and-analytics.md) | Worker job recording, dashboard stats, monitoring |
| [API Standards](./api-standards.md) | API design guidelines |
| [Development](./development.md) | Development workflow and practices |

## Documentation Structure

```
docs/
├── README.md                 # This file
├── getting-started.md        # Quick start guide
├── architecture.md           # System architecture
├── deployment.md             # Deployment guide (all environments)
├── development.md            # Development workflow
├── api-standards.md          # API design standards
├── authentication.md         # Logto/auth integration
├── services/                 # Service-specific docs
│   ├── user-service.md
│   ├── content-service.md
│   └── ...
└── archive/                  # Historical documents
    └── ...
```

## Platform Overview

AuthorWorks is an AI-powered creative content platform built with:

- **Backend**: Rust microservices running on Spin (WebAssembly)
- **Frontend**: Leptos (Rust/WASM) + Next.js options
- **Auth**: Logto (open-source identity platform)
- **Infrastructure**: PostgreSQL, Redis, RabbitMQ, MinIO/S3, Elasticsearch

## Deployment Targets

| Environment | Use Case | Command |
|-------------|----------|---------|
| Local | Development | `./scripts/deploy.sh local --build` |
| Homelab | K3s testing | `./scripts/deploy.sh homelab` |
| EC2 | MVP Production | `./scripts/deploy.sh ec2` |
| EKS | Scalable Production | `./scripts/deploy.sh eks --build` |

## Need Help?

- Check the [Getting Started](./getting-started.md) guide
- Review the [Deployment](./deployment.md) documentation
- File an issue on GitHub

