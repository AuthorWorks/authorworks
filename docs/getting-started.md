# Getting Started with AuthorWorks

This guide will help you get AuthorWorks running locally for development.

## Prerequisites

- Docker & Docker Compose
- Git
- (Optional) Rust 1.83+ with wasm32-wasi target
- (Optional) Node.js 18+ for frontend development

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/AuthorWorks/authorworks.git
cd authorworks
```

### 2. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit with your settings (defaults work for local dev)
vim .env
```

### 3. Deploy Locally

```bash
# Start all services
./scripts/deploy.sh local --build

# Or use make
make deploy-local
```

### 4. Access the Application

| Service | URL | Credentials |
|---------|-----|-------------|
| App | http://localhost:8080 | - |
| Logto (Auth) | http://localhost:3001 | Setup on first visit |
| Logto Admin | http://localhost:3002 | Setup on first visit |
| Grafana | http://localhost:3000 | admin / authorworks123 |
| RabbitMQ | http://localhost:15672 | authorworks / authorworks123 |
| MinIO Console | http://localhost:9001 | authorworks / authorworks123 |
| Mailpit (Dev Email) | http://localhost:8025 | - |

## Setting Up Logto (Authentication)

1. Visit http://localhost:3002 (Logto Admin Console)
2. Complete the initial setup wizard
3. Create an application:
   - Name: `AuthorWorks App`
   - Type: `Traditional Web`
   - Redirect URI: `http://localhost:8080/auth/callback`
4. Copy the Client ID and Client Secret
5. Update your `.env` file:
   ```
   LOGTO_CLIENT_ID=<your-client-id>
   LOGTO_CLIENT_SECRET=<your-client-secret>
   ```
6. Restart services: `./scripts/deploy.sh local`

## Project Structure

```
authorworks/
├── services/           # Microservices (Rust/Spin)
│   ├── user/          # User authentication & profiles
│   ├── content/       # Content management & AI generation
│   ├── storage/       # File storage (S3/MinIO)
│   ├── editor/        # Editor sessions
│   ├── subscription/  # Billing (Stripe)
│   ├── messaging/     # Real-time messaging
│   ├── discovery/     # Search & recommendations
│   └── media/         # Audio/video/image processing
├── workers/           # Background job processors
│   ├── content/       # AI content generation worker
│   └── media/         # Media processing worker
├── frontend/          # Frontend applications
│   └── landing/       # Landing page (Leptos + Next.js)
├── core/              # Shared libraries
│   └── book-generator/# AI book generation engine
├── k8s/               # Kubernetes manifests
├── terraform/         # Infrastructure as Code
├── config/            # Configuration files
├── scripts/           # Deployment & utility scripts
└── docs/              # Documentation
```

## Common Commands

```bash
# Start local development
./scripts/deploy.sh local --build

# Stop local development
./scripts/deploy.sh local --down

# View logs
./scripts/deploy.sh local --logs

# Run health checks
./scripts/deploy.sh local --verify

# Build only (no deploy)
make build

# Run tests
make test
```

## Development Workflow

1. **Make changes** to service code in `services/`
2. **Rebuild** the changed service:
   ```bash
   docker compose build <service-name>
   docker compose up -d <service-name>
   ```
3. **Test** your changes via the API
4. **Check logs** if something isn't working:
   ```bash
   docker compose logs -f <service-name>
   ```

## Troubleshooting

### Services won't start

```bash
# Check Docker is running
docker info

# Check for port conflicts
docker compose ps
lsof -i :8080

# View service logs
docker compose logs <service-name>
```

### Database connection issues

```bash
# Check PostgreSQL is healthy
docker compose exec postgres pg_isready

# View PostgreSQL logs
docker compose logs postgres
```

### Authentication not working

1. Verify Logto is running: http://localhost:3001/api/status
2. Check environment variables are set correctly
3. Ensure redirect URIs match in Logto admin console

## Next Steps

- Read the [Architecture](./architecture.md) documentation
- Review [API Standards](./api-standards.md)
- Check [Deployment](./deployment.md) for production setup

