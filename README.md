# AuthorWorks

AI-powered creative content platform - Monorepo with core engine + 8 microservices (Rust/WebAssembly)

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Quick Start

```bash
# Clone
git clone https://github.com/AuthorWorks/authorworks.git
cd authorworks

# Configure
cp .env.example .env

# Deploy locally
./scripts/deploy.sh local --build

# Access
open http://localhost:8080
```

## Documentation

📚 **[Full Documentation](./docs/README.md)**

| Guide | Description |
|-------|-------------|
| [Getting Started](./docs/getting-started.md) | Setup and first deployment |
| [Architecture](./docs/architecture.md) | System design and components |
| [Deployment](./docs/deployment.md) | Deploy to any environment |
| [Authentication](./docs/authentication.md) | Logto integration |

## Deployment Targets

| Environment | Command | Use Case |
|-------------|---------|----------|
| **Local** | `./scripts/deploy.sh local --build` | Development |
| **Homelab** | `./scripts/deploy.sh homelab` | K3s testing |
| **EC2** | `./scripts/deploy.sh ec2` | MVP production |
| **EKS** | `./scripts/deploy.sh eks --build` | Scalable production |

## Architecture

```
┌─────────────────┐     ┌─────────────────────────────────────────┐
│   Frontend      │────▶│              API Gateway                │
│   (Leptos)      │     │               (Nginx)                   │
└─────────────────┘     └───────────────────┬─────────────────────┘
                                            │
        ┌───────────────────────────────────┼───────────────────────────────────┐
        │                                   │                                   │
   ┌────┴────┐  ┌─────────┐  ┌─────────┐  ┌─┴───────┐  ┌─────────┐  ┌─────────┐
   │  User   │  │ Content │  │ Storage │  │ Editor  │  │  Subsc. │  │  Media  │
   │ Service │  │ Service │  │ Service │  │ Service │  │ Service │  │ Service │
   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘
        │            │            │            │            │            │
        └────────────┴────────────┴─────┬──────┴────────────┴────────────┘
                                        │
   ┌────────────────────────────────────┼────────────────────────────────────┐
   │  PostgreSQL  │   Redis   │  RabbitMQ  │  Elasticsearch  │   MinIO/S3   │
   └──────────────┴───────────┴────────────┴─────────────────┴──────────────┘
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Frontend | Leptos (Rust/WASM) |
| Services | Rust + Spin (WASM) |
| Auth | Logto |
| Database | PostgreSQL |
| Cache | Redis |
| Queue | RabbitMQ |
| Search | Elasticsearch |
| Storage | MinIO / S3 |

## Project Structure

```
authorworks/
├── services/           # 8 microservices
│   ├── user/          # Authentication & profiles
│   ├── content/       # AI content generation
│   ├── storage/       # File storage
│   ├── editor/        # Editing sessions
│   ├── subscription/  # Billing (Stripe)
│   ├── messaging/     # Real-time messaging
│   ├── discovery/     # Search & recommendations
│   └── media/         # Audio/video/image
├── workers/           # Background processors
├── frontend/          # Leptos + Next.js
├── core/              # Shared libraries
├── k8s/               # Kubernetes manifests
├── terraform/         # AWS infrastructure
├── config/            # Configuration files
├── scripts/           # Deployment scripts
└── docs/              # Documentation
```

## Services (Ports)

| Service | Port | Description |
|---------|------|-------------|
| API Gateway | 8080 | Main entry point |
| User | 3101 | Auth & profiles |
| Content | 3102 | Stories & AI generation |
| Storage | 3103 | File management |
| Editor | 3104 | Edit sessions |
| Subscription | 3105 | Billing |
| Messaging | 3106 | WebSocket |
| Discovery | 3107 | Search |
| Media | 3108 | Media processing |
| Logto | 3001/3002 | Auth provider |

## Development

```bash
# Start local environment
./scripts/deploy.sh local --build

# View logs
./scripts/deploy.sh local --logs

# Health check
./scripts/deploy.sh local --verify

# Stop
./scripts/deploy.sh local --down
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.
