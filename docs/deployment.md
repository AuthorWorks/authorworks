# AuthorWorks Deployment Guide

This guide covers deploying AuthorWorks to all supported environments.

## Deployment Environments

| Environment | Description | Infrastructure |
|-------------|-------------|----------------|
| **Local** | Development on your machine | Docker Compose |
| **Homelab** | K3s cluster at home | Docker Compose + Traefik |
| **EC2** | AWS EC2 MVP production | Docker Compose + Nginx |
| **EKS** | AWS EKS scalable production | Kubernetes + Terraform |

## Unified Deploy Script

All deployments use the unified script:

```bash
./scripts/deploy.sh [environment] [options]
```

### Options

| Option | Description |
|--------|-------------|
| `--build` | Build services before deploying |
| `--no-build` | Skip building (use existing images) |
| `--down` | Tear down the deployment |
| `--logs` | Follow logs after deployment |
| `--verify` | Run health checks after deployment |
| `--help` | Show help message |

---

## Local Development

### Prerequisites
- Docker & Docker Compose

### Deploy
```bash
# First time (builds images)
./scripts/deploy.sh local --build

# Subsequent runs (faster)
./scripts/deploy.sh local

# With verification
./scripts/deploy.sh local --verify
```

### Services
| Service | URL |
|---------|-----|
| Application | http://localhost:8080 |
| Logto Auth | http://localhost:3001 |
| Logto Admin | http://localhost:3002 |
| Grafana | http://localhost:3000 |
| Prometheus | http://localhost:9090 |
| RabbitMQ | http://localhost:15672 |
| MinIO | http://localhost:9001 |
| Mailpit | http://localhost:8025 |

### Teardown
```bash
./scripts/deploy.sh local --down
```

---

## Homelab (K3s)

### Prerequisites
- K3s cluster running
- Traefik ingress controller
- External network `llm_network`
- Existing services: PostgreSQL, Redis, MinIO

### Environment Variables
```bash
export DOMAIN=leopaska.xyz
export POSTGRES_PASSWORD=your-password
export REDIS_PASSWORD=your-password
export JWT_SECRET=$(openssl rand -base64 32)
export LOGTO_CLIENT_ID=authorworks-app
export LOGTO_CLIENT_SECRET=from-logto-console
```

### Deploy
```bash
./scripts/deploy.sh homelab --build
```

### Services
| Service | URL |
|---------|-----|
| Application | https://authorworks.leopaska.xyz |
| Logto Auth | https://auth.authorworks.leopaska.xyz |
| Logto Admin | https://auth-admin.authorworks.leopaska.xyz |

### SSH Access
```bash
# Direct (local network)
ssh alef  # 192.168.1.200:2222

# Via Cloudflare (anywhere)
ssh homelab  # ssh.leopaska.xyz
```

---

## EC2 Production (MVP)

### Prerequisites
- EC2 instance (t3.large or larger)
- AWS RDS PostgreSQL
- AWS ElastiCache Redis
- AWS S3 bucket
- Domain with DNS pointing to EC2

### Setup EC2 Instance
```bash
# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Clone repository
git clone https://github.com/AuthorWorks/authorworks.git
cd authorworks
```

### Environment Variables
```bash
export DOMAIN=authorworks.io
export DATABASE_URL=postgres://user:pass@rds-endpoint:5432/authorworks
export REDIS_URL=redis://elasticache-endpoint:6379
export S3_BUCKET=authorworks-content
export AWS_REGION=us-west-2
export JWT_SECRET=$(openssl rand -base64 32)
export LOGTO_DATABASE_URL=postgres://user:pass@rds-endpoint:5432/logto
export LOGTO_CLIENT_ID=authorworks-app
export LOGTO_CLIENT_SECRET=from-logto-console
```

### SSL Certificates
```bash
# Install certbot
sudo apt install certbot

# Get certificates
sudo certbot certonly --standalone \
  -d authorworks.io \
  -d www.authorworks.io \
  -d auth.authorworks.io \
  -d auth-admin.authorworks.io
```

### Deploy
```bash
./scripts/deploy.sh ec2 --build
```

### CloudWatch Logging
Logs are automatically sent to CloudWatch Log Groups:
- `/authorworks/nginx`
- `/authorworks/services`
- `/authorworks/workers`
- `/authorworks/logto`

---

## AWS EKS (Production)

### Prerequisites
- AWS CLI configured
- Terraform installed
- kubectl installed
- Helm installed

### Configure Terraform
```bash
cd terraform/aws
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your values
```

### Deploy
```bash
./scripts/deploy.sh eks --build
```

This will:
1. Provision VPC, EKS cluster, RDS, ElastiCache, S3
2. Install SpinKube operator
3. Build and push Docker images to ECR
4. Deploy Kubernetes resources

### Scaling
```bash
# Manual scaling
kubectl scale -n authorworks spinapp/authorworks-platform --replicas=10

# Or via Makefile
make scale-up    # 10 replicas
make scale-down  # 3 replicas
```

### Monitoring
```bash
# View pods
kubectl get pods -n authorworks

# View logs
kubectl logs -n authorworks -l app=authorworks -f

# Port-forward Grafana
kubectl port-forward -n authorworks svc/grafana 3000:3000
```

---

## Infrastructure Components

### All Environments Include

| Component | Local | Homelab | EC2 | EKS |
|-----------|-------|---------|-----|-----|
| PostgreSQL | Container | External | RDS | RDS |
| Redis | Container | External | ElastiCache | ElastiCache |
| MinIO/S3 | Container | External | S3 | S3 |
| RabbitMQ | Container | Container | Container | Amazon MQ |
| Elasticsearch | Container | Container | Container | OpenSearch |
| Logto | Container | Container | Container | Container |
| Prometheus | Container | External | CloudWatch | CloudWatch |
| Grafana | Container | External | - | - |
| Loki | Container | Container | CloudWatch | CloudWatch |

### Microservices (8)

1. **user-service** - Authentication, profiles, preferences
2. **content-service** - Story/book management, AI generation
3. **storage-service** - File upload/download
4. **editor-service** - Editing sessions
5. **subscription-service** - Stripe billing
6. **messaging-service** - WebSocket messaging
7. **discovery-service** - Search, recommendations
8. **media-service** - Audio/video/image processing

### Background Workers (2)

1. **content-worker** - AI content generation jobs
2. **media-worker** - Media processing jobs

---

## Health Checks

### Manual Verification
```bash
# Check gateway health
curl http://localhost:8080/health

# Check individual services
curl http://localhost:8080/api/users/health
curl http://localhost:8080/api/content/health
curl http://localhost:8080/api/storage/health

# Check Logto
curl http://localhost:3001/api/status
```

### Automated Verification
```bash
./scripts/deploy.sh [environment] --verify
```

---

## Troubleshooting

### Common Issues

**Services not starting**
```bash
docker compose logs <service-name>
docker compose ps
```

**Database connection failed**
```bash
# Check PostgreSQL
docker compose exec postgres pg_isready -U authorworks
```

**Auth not working**
1. Check Logto is running: `curl http://localhost:3001/api/status`
2. Verify redirect URIs in Logto admin
3. Check LOGTO_CLIENT_ID and LOGTO_CLIENT_SECRET

**EKS pods not starting**
```bash
kubectl describe pod -n authorworks <pod-name>
kubectl logs -n authorworks <pod-name>
```

### Rollback

```bash
# Docker Compose
docker compose down
git checkout <previous-commit>
./scripts/deploy.sh [environment] --build

# EKS
kubectl rollout undo -n authorworks deployment/authorworks-platform
# Or via Makefile
make rollback
```

