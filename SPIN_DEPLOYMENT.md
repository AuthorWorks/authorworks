# AuthorWorks SPIN WebAssembly Deployment Guide

## Overview

This guide covers the conversion of AuthorWorks platform from traditional container-based microservices to SPIN-based WebAssembly applications, deployable on both Homelab K3s clusters and AWS EKS with auto-scaling capabilities.

## Architecture

### Components
- **SPIN Framework**: Serverless WebAssembly runtime
- **SpinKube**: Kubernetes operator for managing Spin applications
- **Multi-tenancy**: Isolated namespaces for different tenants
- **Auto-scaling**: HPA-based scaling for both homelab and AWS

## Prerequisites

### For Homelab Deployment
- K3s cluster (v1.28+)
- kubectl configured
- Helm 3.x
- Spin CLI
- Docker registry (local or remote)

### For AWS Deployment
- AWS CLI configured
- Terraform 1.5+
- kubectl
- Helm 3.x
- Docker

## Homelab Deployment

### 1. Install K3s with SpinKube Support

```bash
# Install K3s with containerd configuration for Spin
curl -sfL https://get.k3s.io | sh -s - \
  --disable traefik \
  --write-kubeconfig-mode 644

# Configure containerd for Spin runtime
sudo tee /var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl <<EOF
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
  runtime_type = "io.containerd.spin.v2"
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin.options]
  BinaryName = "/usr/local/bin/containerd-shim-spin-v2"
EOF

sudo systemctl restart k3s
```

### 2. Deploy SpinKube Operator

```bash
# Add SpinKube Helm repository
helm repo add spinkube https://spinkube.github.io/charts
helm repo update

# Install Spin operator
helm install spin-operator spinkube/spin-operator \
  --namespace spin-system \
  --create-namespace \
  --version 0.2.0

# Install containerd-shim-spin
helm install containerd-shim-spin spinkube/containerd-shim-spin-installer \
  --namespace spin-system \
  --version 0.14.1
```

### 3. Deploy AuthorWorks

```bash
# Run deployment script
chmod +x scripts/deploy-homelab.sh
./scripts/deploy-homelab.sh

# Or manually:
kubectl apply -f k8s/
```

### 4. Access the Application

Configure your `/etc/hosts`:
```
192.168.1.100  authorworks.homelab.local api.authorworks.homelab.local
192.168.1.100  tenant1.authorworks.homelab.local tenant2.authorworks.homelab.local
```

## AWS Deployment

### 1. Infrastructure Setup

```bash
cd terraform/aws

# Initialize Terraform
terraform init

# Plan deployment
terraform plan -var="environment=production"

# Apply infrastructure
terraform apply -var="environment=production"
```

### 2. Build and Push Spin Application

```bash
# Build Spin app
spin build

# Build container image
docker build -f Dockerfile.spin -t authorworks-spin:latest .

# Push to ECR
aws ecr get-login-password --region us-west-2 | docker login --username AWS --password-stdin <ECR_URL>
docker tag authorworks-spin:latest <ECR_URL>/authorworks-spin:latest
docker push <ECR_URL>/authorworks-spin:latest
```

### 3. Deploy with Helm

```bash
# Deploy AuthorWorks
helm install authorworks charts/authorworks \
  --namespace authorworks \
  --create-namespace \
  --values charts/authorworks/values-production.yaml
```

## Multi-Tenant Configuration

### Tenant Isolation
Each tenant gets:
- Dedicated namespace
- Isolated database schema
- Separate Redis database
- S3 bucket prefix
- Resource quotas

### Adding New Tenants

1. Update Helm values:
```yaml
multiTenancy:
  tenants:
    - name: new-tenant
      namespace: authorworks-new-tenant
      replicas: 2
      database:
        name: new_tenant_db
```

2. Apply changes:
```bash
helm upgrade authorworks charts/authorworks
```

## Monitoring & Scaling

### Metrics
- Prometheus ServiceMonitor included
- Grafana dashboards available
- Custom metrics for WebAssembly performance

### Auto-scaling Configuration
```yaml
autoscaling:
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - cpu: 70%
    - memory: 80%
    - requests-per-second: 1000
```

## Service Migration Guide

### Converting Services to SPIN

1. **Update Cargo.toml** for WASM target:
```toml
[dependencies]
spin-sdk = "2.0"
# Remove tokio, use spin's async runtime
```

2. **Modify main.rs**:
```rust
use spin_sdk::http::{Request, Response};

#[spin_sdk::http_component]
fn handle_request(req: Request) -> Response {
    // Your handler logic
}
```

3. **Build for WASM**:
```bash
cargo build --target wasm32-wasi --release
```

## Performance Optimization

### WASM Optimization
- Use `wasm-opt` for size reduction
- Enable LTO in release builds
- Minimize dependencies

### Caching Strategy
- Redis for session data
- CDN for static assets
- Edge caching for API responses

## Troubleshooting

### Common Issues

1. **Spin app not starting**:
   ```bash
   kubectl logs -n authorworks -l app=authorworks
   kubectl describe spinapp authorworks-platform -n authorworks
   ```

2. **Performance issues**:
   - Check resource limits
   - Verify WASM optimization
   - Review network policies

3. **Multi-tenant isolation**:
   - Verify namespace policies
   - Check RBAC configuration
   - Review network segmentation

## Security Considerations

- All WASM modules run in sandboxed environment
- Network policies enforce tenant isolation
- Secrets managed via Kubernetes secrets
- TLS termination at ingress

## Backup and Recovery

### Database Backup
```bash
# Automated daily backups to S3
kubectl apply -f k8s/backup-cronjob.yaml
```

### Disaster Recovery
- Multi-region deployment supported
- Point-in-time recovery for RDS
- Automated failover configuration

## Cost Optimization

### AWS Cost Savings
- Spot instances for non-critical workloads
- Reserved instances for baseline capacity
- S3 lifecycle policies for storage optimization
- WebAssembly reduces compute requirements by ~40%

### Homelab Efficiency
- Lower memory footprint
- Faster cold starts
- Reduced CPU usage
- Better resource utilization

## Next Steps

1. Configure CI/CD pipeline
2. Set up monitoring dashboards
3. Implement backup strategy
4. Configure alerts
5. Performance tuning

## Support

For issues or questions:
- GitHub Issues: https://github.com/authorworks/platform
- Documentation: https://docs.authorworks.io
- Community: https://discord.gg/authorworks