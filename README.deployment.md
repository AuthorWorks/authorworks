# AuthorWorks Deployment Guide

## Overview

AuthorWorks is a production-ready AI-assisted content creation platform built with SPIN WebAssembly for optimal performance and cost efficiency. This guide covers deployment to both K3S homelab clusters and AWS EKS production environments.

## Architecture

- **SPIN WebAssembly**: Serverless microservices with sub-millisecond cold starts
- **Containerized Services**: Fallback support for services requiring full OS capabilities
- **Multi-tenancy**: Isolated tenant deployments with resource quotas
- **Auto-scaling**: Horizontal pod autoscaling based on CPU/memory metrics
- **Observability**: Full metrics, logging, and tracing integration

## Prerequisites

### Tools Required
- `kubectl` (v1.28+)
- `helm` (v3.13+)
- `spin` CLI (v2.0+)
- `docker` (v24.0+)
- `rustc` (v1.80+) with `wasm32-wasip1` target
- `make` (GNU Make 4.0+)

### Cluster Requirements
- Kubernetes 1.28+ with SpinKube operator support
- Minimum 8 CPU cores, 16GB RAM for homelab
- Network policies and RBAC enabled
- Ingress controller (nginx-ingress recommended)
- cert-manager for TLS certificates

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/authorworks/authorworks-platform.git
cd authorworks-platform
cp .env.example .env
# Edit .env with your configuration
```

### 2. Verify Environment

```bash
make verify
```

This will check all prerequisites and report any issues.

### 3. Build Application

```bash
# Standard build
make build-spin

# Optimized build (recommended for production)
make build-optimized
```

### 4. Deploy to Homelab

```bash
make deploy-homelab
```

This will:
- Install SpinKube operator if not present
- Build and optimize WASM modules
- Deploy all Kubernetes resources
- Configure ingress and TLS
- Set up monitoring

### 5. Deploy to Production (AWS EKS)

```bash
# Configure AWS credentials
export AWS_PROFILE=production

# Deploy
make deploy-aws
```

## Configuration

### Environment Variables

Key environment variables (see `.env.example`):

- `DATABASE_URL`: PostgreSQL connection string
- `REDIS_URL`: Redis connection for caching/sessions
- `MINIO_ENDPOINT`: Object storage endpoint
- `JWT_SECRET`: Secret for JWT token signing
- `ALLOWED_ORIGINS`: CORS allowed origins

### Resource Limits

Default resource allocations per service:
- CPU: 250m-2000m
- Memory: 512Mi-4Gi
- WASM Memory: 4GB max

Adjust in `k8s/spinapp.yaml` based on your workload.

### Multi-tenancy

Each tenant gets:
- Isolated namespace
- Resource quotas (CPU, memory, storage)
- Network policies preventing cross-tenant communication
- Separate database schemas
- Independent scaling policies

## Monitoring & Observability

### Metrics

Prometheus metrics available at `/metrics`:
- Request rate and latency
- Error rates
- WASM memory usage
- Cache hit rates
- Active connections

### Health Checks

All services expose health endpoints:
```bash
curl https://your-domain/health
curl https://your-domain/api/{service}/health
```

### Logs

View logs:
```bash
make logs
```

Or for specific service:
```bash
kubectl logs -n authorworks -l component=user-service -f
```

## Operations

### Scaling

Manual scaling:
```bash
make scale-up   # Scale to 10 replicas
make scale-down  # Scale to 3 replicas
```

Auto-scaling is configured via HPA with:
- Min replicas: 3
- Max replicas: 20
- CPU target: 70%
- Memory target: 80%

### Rolling Updates

```bash
# Build and deploy new version
git tag v1.0.1
make build-optimized push
kubectl set image -n authorworks spinapp/authorworks-platform \
  authorworks-platform=ghcr.io/authorworks/authorworks-platform:v1.0.1
```

### Rollback

```bash
make rollback
```

### Backup & Recovery

Database backups:
```bash
kubectl exec -n authorworks deployment/postgresql -- \
  pg_dump -U postgres authorworks > backup.sql
```

## Security

### Network Policies

- Default deny all ingress/egress
- Explicit allow rules for service communication
- Tenant isolation enforced
- External API calls restricted

### Secrets Management

- Kubernetes secrets for sensitive data
- Environment-specific secret overlays
- Rotation support via external-secrets operator

### Pod Security

- Non-root containers
- Read-only root filesystem
- Dropped capabilities
- Security contexts enforced

## Troubleshooting

### Common Issues

1. **WASM build failures**
   ```bash
   rustup target add wasm32-wasip1
   ```

2. **SpinKube CRDs not found**
   ```bash
   helm repo add spinkube https://spinkube.github.io/charts
   helm install spin-operator spinkube/spin-operator -n spin-system
   ```

3. **Service unhealthy**
   ```bash
   kubectl describe spinapp -n authorworks authorworks-platform
   kubectl logs -n authorworks -l app=authorworks --tail=100
   ```

4. **Performance issues**
   ```bash
   # Check resource usage
   kubectl top pods -n authorworks
   # Run benchmarks
   make benchmark
   ```

### Debug Mode

Enable debug logging:
```bash
kubectl set env -n authorworks spinapp/authorworks-platform LOG_LEVEL=debug
```

## Performance Optimization

### WASM Optimization

The build process includes:
- Dead code elimination
- Size optimization with wasm-opt
- Aggressive inlining
- Link-time optimization (LTO)

### Caching Strategy

- Redis for session and API caching
- CDN for static assets
- Database connection pooling
- HTTP caching headers

### Resource Tuning

Monitor and adjust:
```bash
# View current usage
kubectl top pods -n authorworks

# Adjust limits
kubectl edit spinapp -n authorworks authorworks-platform
```

## Production Checklist

Before going to production:

- [ ] All health checks passing
- [ ] TLS certificates configured
- [ ] Backup strategy in place
- [ ] Monitoring dashboards set up
- [ ] Alerting rules configured
- [ ] Resource quotas appropriate
- [ ] Network policies reviewed
- [ ] Secrets rotated from defaults
- [ ] Performance benchmarks met
- [ ] Disaster recovery tested

## Support

For issues or questions:
- GitHub Issues: https://github.com/authorworks/authorworks-platform/issues
- Documentation: https://docs.authorworks.io
- Community Discord: https://discord.gg/authorworks

## License

Copyright (c) 2024 AuthorWorks. All rights reserved.