# AuthorWorks Deployment Guide

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Local Development](#local-development)
3. [Production Deployment](#production-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [CI/CD Pipeline](#cicd-pipeline)
6. [Monitoring & Observability](#monitoring--observability)
7. [Backup & Recovery](#backup--recovery)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Tools
- Docker 20.10+ and Docker Compose 2.0+
- Kubernetes 1.25+ (for production)
- Helm 3.0+ (for K8s deployments)
- Terraform 1.0+ (for infrastructure)
- Git
- Make (optional but recommended)

### Required Accounts
- AWS/GCP/Azure account (for cloud deployment)
- Docker Hub account (for image registry)
- GitHub account (for code repository)
- Stripe account (for payments)
- SendGrid/AWS SES (for emails)

## Local Development

### Quick Start
```bash
# Clone the repository
git clone https://github.com/authorworks/authorworks.git
cd authorworks

# Copy environment files
for service in authorworks-*/; do
  cp "$service/.env.example" "$service/.env"
done

# Start all services
docker-compose up -d

# Check service health
docker-compose ps
curl http://localhost:8080/health

# View logs
docker-compose logs -f [service-name]
```

### Service URLs (Local)
- API Gateway: http://localhost:8080
- UI Shell: http://localhost:3000
- RabbitMQ Management: http://localhost:15672
- MinIO Console: http://localhost:9001
- PostgreSQL: localhost:5432
- Redis: localhost:6379

### Building Individual Services
```bash
# Build a specific service
cd authorworks-[service-name]
cargo build --release

# Run tests
cargo test

# Run with hot reload
cargo watch -x run
```

## Production Deployment

### Environment Setup

#### 1. Infrastructure Provisioning
```bash
# Navigate to infrastructure directory
cd infrastructure/terraform

# Initialize Terraform
terraform init

# Review plan
terraform plan -var-file=production.tfvars

# Apply infrastructure
terraform apply -var-file=production.tfvars
```

#### 2. Database Setup
```bash
# Connect to production database
psql -h $DB_HOST -U $DB_USER -d authorworks

# Run migrations
for service in authorworks-*/; do
  cd "$service"
  sqlx migrate run
  cd ..
done
```

#### 3. Environment Variables
Create `.env.production` for each service with production values:
```bash
# Example for user-service
DATABASE_URL=postgresql://user:pass@prod-db:5432/authorworks
REDIS_URL=redis://prod-redis:6379
JWT_SECRET=<secure-random-string>
ENVIRONMENT=production
LOG_LEVEL=info
```

### Docker Deployment

#### Building Images
```bash
# Build all services
make build-all

# Or build individually
docker build -t authorworks/user-service:latest ./authorworks-user-service
docker build -t authorworks/content-service:latest ./authorworks-content-service
# ... repeat for all services
```

#### Pushing to Registry
```bash
# Login to registry
docker login

# Tag images
docker tag authorworks/user-service:latest authorworks/user-service:v1.0.0

# Push images
docker push authorworks/user-service:v1.0.0
# ... repeat for all services
```

### Docker Swarm Deployment
```bash
# Initialize swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.production.yml authorworks

# Check deployment
docker stack services authorworks
docker service logs authorworks_user-service
```

## Kubernetes Deployment

### Namespace Setup
```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: authorworks
```

### ConfigMaps and Secrets
```bash
# Create secrets
kubectl create secret generic db-credentials \
  --from-literal=username=$DB_USER \
  --from-literal=password=$DB_PASSWORD \
  -n authorworks

kubectl create secret generic jwt-secret \
  --from-literal=secret=$JWT_SECRET \
  -n authorworks
```

### Helm Deployment
```bash
# Add Helm repository
helm repo add authorworks https://charts.authorworks.io
helm repo update

# Install with custom values
helm install authorworks authorworks/authorworks \
  --namespace authorworks \
  --values values.production.yaml

# Upgrade deployment
helm upgrade authorworks authorworks/authorworks \
  --namespace authorworks \
  --values values.production.yaml
```

### Service Deployment Example
```yaml
# k8s/deployments/user-service.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: user-service
  namespace: authorworks
spec:
  replicas: 3
  selector:
    matchLabels:
      app: user-service
  template:
    metadata:
      labels:
        app: user-service
    spec:
      containers:
      - name: user-service
        image: authorworks/user-service:v1.0.0
        ports:
        - containerPort: 3001
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: url
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 3001
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 3001
          initialDelaySeconds: 10
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: user-service
  namespace: authorworks
spec:
  selector:
    app: user-service
  ports:
  - port: 3001
    targetPort: 3001
  type: ClusterIP
```

### Ingress Configuration
```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: authorworks-ingress
  namespace: authorworks
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
spec:
  tls:
  - hosts:
    - api.authorworks.io
    secretName: authorworks-tls
  rules:
  - host: api.authorworks.io
    http:
      paths:
      - path: /api/users
        pathType: Prefix
        backend:
          service:
            name: user-service
            port:
              number: 3001
      - path: /api/content
        pathType: Prefix
        backend:
          service:
            name: content-service
            port:
              number: 3002
```

## CI/CD Pipeline

### GitHub Actions Workflow
```yaml
# .github/workflows/deploy.yml
name: Deploy to Production

on:
  push:
    branches: [main]
  release:
    types: [created]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - run: cargo test --all

  build-and-push:
    needs: test
    runs-on: ubuntu-latest
    strategy:
      matrix:
        service: [user, content, storage, editor, subscription, messaging, discovery, audio, video, graphics]
    steps:
    - uses: actions/checkout@v3
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    
    - name: Login to DockerHub
      uses: docker/login-action@v2
      with:
        username: ${{ secrets.DOCKER_USERNAME }}
        password: ${{ secrets.DOCKER_PASSWORD }}
    
    - name: Build and push
      uses: docker/build-push-action@v4
      with:
        context: ./authorworks-${{ matrix.service }}-service
        push: true
        tags: |
          authorworks/${{ matrix.service }}-service:latest
          authorworks/${{ matrix.service }}-service:${{ github.sha }}
        cache-from: type=gha
        cache-to: type=gha,mode=max

  deploy:
    needs: build-and-push
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup kubectl
      uses: azure/setup-kubectl@v3
    
    - name: Configure AWS credentials
      uses: aws-actions/configure-aws-credentials@v2
      with:
        aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
        aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        aws-region: us-west-2
    
    - name: Update kubeconfig
      run: aws eks update-kubeconfig --name authorworks-cluster
    
    - name: Deploy to Kubernetes
      run: |
        kubectl set image deployment/user-service user-service=authorworks/user-service:${{ github.sha }} -n authorworks
        kubectl set image deployment/content-service content-service=authorworks/content-service:${{ github.sha }} -n authorworks
        # ... repeat for all services
        
        kubectl rollout status deployment/user-service -n authorworks
        kubectl rollout status deployment/content-service -n authorworks
        # ... repeat for all services
```

## Monitoring & Observability

### Prometheus Setup
```yaml
# monitoring/prometheus-values.yaml
prometheus:
  prometheusSpec:
    serviceMonitorSelectorNilUsesHelmValues: false
    podMonitorSelectorNilUsesHelmValues: false
    retention: 30d
    storageSpec:
      volumeClaimTemplate:
        spec:
          storageClassName: gp2
          resources:
            requests:
              storage: 100Gi
```

### Grafana Dashboards
```bash
# Install Prometheus and Grafana
helm install monitoring prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace \
  --values monitoring/prometheus-values.yaml

# Access Grafana
kubectl port-forward -n monitoring svc/monitoring-grafana 3000:80
# Default: admin/prom-operator
```

### Service Metrics
Each service exposes metrics at `/metrics`:
```rust
// Example metrics endpoint
use prometheus::{Encoder, TextEncoder, Counter, Histogram};

lazy_static! {
    static ref REQUEST_COUNT: Counter = Counter::new(
        "authorworks_requests_total", 
        "Total number of requests"
    ).unwrap();
    
    static ref REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "authorworks_request_duration_seconds",
            "Request duration in seconds"
        )
    ).unwrap();
}
```

### Logging with ELK Stack
```bash
# Deploy Elasticsearch
helm install elasticsearch elastic/elasticsearch \
  --namespace logging \
  --set replicas=3 \
  --set minimumMasterNodes=2

# Deploy Kibana
helm install kibana elastic/kibana \
  --namespace logging \
  --set elasticsearch.hosts[0]=http://elasticsearch-master:9200

# Deploy Filebeat
helm install filebeat elastic/filebeat \
  --namespace logging \
  --values monitoring/filebeat-values.yaml
```

## Backup & Recovery

### Database Backups
```bash
#!/bin/bash
# backup-database.sh

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="authorworks_backup_${TIMESTAMP}.sql"

# Perform backup
pg_dump -h $DB_HOST -U $DB_USER -d authorworks > $BACKUP_FILE

# Compress backup
gzip $BACKUP_FILE

# Upload to S3
aws s3 cp ${BACKUP_FILE}.gz s3://authorworks-backups/postgres/

# Clean up old backups (keep 30 days)
aws s3 ls s3://authorworks-backups/postgres/ | \
  while read -r line; do
    createDate=$(echo $line | awk '{print $1" "$2}')
    createDate=$(date -d "$createDate" +%s)
    olderThan=$(date -d "30 days ago" +%s)
    if [[ $createDate -lt $olderThan ]]; then
      fileName=$(echo $line | awk '{print $4}')
      aws s3 rm s3://authorworks-backups/postgres/$fileName
    fi
  done
```

### Disaster Recovery Plan
1. **RTO (Recovery Time Objective)**: 4 hours
2. **RPO (Recovery Point Objective)**: 1 hour

#### Recovery Steps:
```bash
# 1. Restore database
gunzip authorworks_backup_latest.sql.gz
psql -h $NEW_DB_HOST -U $DB_USER -d authorworks < authorworks_backup_latest.sql

# 2. Update DNS
aws route53 change-resource-record-sets \
  --hosted-zone-id $ZONE_ID \
  --change-batch file://dns-failover.json

# 3. Scale up services
kubectl scale deployment --all --replicas=3 -n authorworks

# 4. Verify health
./scripts/verify-deployment.sh
```

## Troubleshooting

### Common Issues

#### 1. Service Won't Start
```bash
# Check logs
kubectl logs -n authorworks deployment/user-service --tail=100

# Check events
kubectl get events -n authorworks --sort-by='.lastTimestamp'

# Describe pod
kubectl describe pod -n authorworks user-service-xxxxx
```

#### 2. Database Connection Issues
```bash
# Test connection
kubectl run -it --rm debug --image=postgres:14 --restart=Never -- \
  psql -h postgres-service -U authorworks -c "SELECT 1"

# Check secrets
kubectl get secret db-credentials -n authorworks -o yaml
```

#### 3. High Memory Usage
```bash
# Check resource usage
kubectl top pods -n authorworks

# Get memory limits
kubectl describe deployment user-service -n authorworks | grep -A 3 "Limits:"

# Increase limits if needed
kubectl set resources deployment user-service \
  --limits=memory=1Gi -n authorworks
```

#### 4. Slow Performance
```bash
# Check metrics
curl http://localhost:8080/metrics | grep -E "request_duration|request_count"

# Enable debug logging
kubectl set env deployment/user-service LOG_LEVEL=debug -n authorworks

# Check database slow queries
psql -h $DB_HOST -U $DB_USER -d authorworks -c \
  "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10"
```

### Health Check Endpoints
All services expose health endpoints:
- `/health` - Detailed health status
- `/health/live` - Liveness probe (is service running?)
- `/health/ready` - Readiness probe (can service accept traffic?)

### Support Contacts
- **DevOps Team**: devops@authorworks.io
- **On-Call**: Use PagerDuty
- **Slack**: #authorworks-ops
- **Documentation**: https://docs.authorworks.io

## Security Considerations

### SSL/TLS Setup
```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.11.0/cert-manager.yaml

# Create ClusterIssuer
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@authorworks.io
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF
```

### Network Policies
```yaml
# k8s/network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: authorworks-network-policy
  namespace: authorworks
spec:
  podSelector: {}
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: authorworks
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: authorworks
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: TCP
      port: 53
    - protocol: UDP
      port: 53
```

## Performance Optimization

### Caching Strategy
- Redis for session management
- CDN for static assets
- Database query caching
- Application-level caching

### Auto-scaling
```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: user-service-hpa
  namespace: authorworks
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: user-service
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

This deployment guide provides comprehensive instructions for deploying AuthorWorks in various environments, from local development to production Kubernetes clusters.