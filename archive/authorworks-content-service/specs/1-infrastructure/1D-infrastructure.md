# Technical Specification: 1D - Infrastructure Setup

## Overview

This specification details the cloud infrastructure and Kubernetes setup required for the AuthorWorks platform across development, staging, and production environments.

## Objectives

- Establish scalable, resilient infrastructure for the microservices architecture
- Create consistent environments across development, staging, and production
- Implement infrastructure as code for reproducibility and version control
- Set up observability and monitoring infrastructure
- Ensure security best practices are implemented

## Requirements

### 1. Infrastructure as Code

Implement all infrastructure using Terraform with the following structure:

#### Repository Organization

Create an `authorworks-infra` repository with the following structure:

```
authorworks-infra/
├── modules/
│   ├── vpc/            # Network configuration
│   ├── eks/            # Kubernetes cluster
│   ├── rds/            # PostgreSQL databases
│   ├── s3/             # Object storage
│   ├── redis/          # Caching layer
│   ├── cdn/            # Content delivery
│   ├── dns/            # DNS configuration
│   └── monitoring/     # Observability stack
├── environments/
│   ├── dev/            # Development environment
│   ├── staging/        # Staging environment
│   └── production/     # Production environment
├── providers/          # Provider configurations
├── variables/          # Shared variables
└── scripts/            # Utility scripts
```

#### Terraform Module Requirements

Each module should include:

1. Input and output variables with descriptions
2. Complete documentation in README.md
3. Validation rules for input variables
4. Local values for computed values
5. Tagging strategy for resources

Example VPC module:

```hcl
variable "project" {
  description = "Project name"
  type        = string
  default     = "authorworks"
}

variable "environment" {
  description = "Environment name"
  type        = string
  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be one of: dev, staging, production."
  }
}

variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

locals {
  name_prefix = "${var.project}-${var.environment}"
  
  tags = {
    Project     = var.project
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true
  
  tags = merge(
    local.tags,
    {
      Name = "${local.name_prefix}-vpc"
    }
  )
}

# Additional resources...

output "vpc_id" {
  description = "ID of the VPC"
  value       = aws_vpc.main.id
}

# Additional outputs...
```

### 2. Kubernetes Cluster Setup

Configure Kubernetes clusters using Amazon EKS with the following specifications:

#### Cluster Configuration

1. **Node Groups**:
   - System node group (for core services)
   - Application node group (for business services)
   - Spot instance node group (for batch processing)

2. **Networking**:
   - VPC with private and public subnets
   - Network policies for service isolation
   - Service mesh for secure service-to-service communication

3. **Cluster Autoscaling**:
   - Horizontal pod autoscaling
   - Cluster autoscaler for node scaling
   - Spot instance integration for cost optimization

#### Kubernetes Manifests Structure

Organize Kubernetes manifests in a separate `authorworks-k8s` repository:

```
authorworks-k8s/
├── base/
│   ├── namespaces/
│   ├── system/
│   │   ├── ingress-nginx/
│   │   ├── cert-manager/
│   │   ├── external-dns/
│   │   ├── linkerd/
│   │   └── monitoring/
│   └── applications/
│       ├── authorworks-gateway/
│       ├── authorworks-user-service/
│       ├── authorworks-content/
│       ├── authorworks-editor/
│       └── ...
├── overlays/
│   ├── dev/
│   ├── staging/
│   └── production/
└── scripts/
```

Use Kustomize for environment-specific configurations:

```yaml
# base/applications/authorworks-gateway/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: authorworks-gateway
spec:
  replicas: 2
  selector:
    matchLabels:
      app: authorworks-gateway
  template:
    metadata:
      labels:
        app: authorworks-gateway
    spec:
      containers:
      - name: authorworks-gateway
        image: authorworks/gateway:latest
        resources:
          requests:
            memory: "256Mi"
            cpu: "200m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        ports:
        - containerPort: 8080
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /readiness
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

```yaml
# overlays/production/authorworks-gateway/kustomization.yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
- ../../../base/applications/authorworks-gateway
patchesStrategicMerge:
- deployment-patch.yaml
```

```yaml
# overlays/production/authorworks-gateway/deployment-patch.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: authorworks-gateway
spec:
  replicas: 5
  template:
    spec:
      containers:
      - name: authorworks-gateway
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
```

### 3. Database Infrastructure

Set up PostgreSQL databases with the following requirements:

#### Database Architecture

1. **Service-Specific Databases**:
   - Create separate databases for each service
   - Use RDS PostgreSQL with multi-AZ deployment
   - Configure appropriate instance sizes per environment

2. **Backup Strategy**:
   - Automated daily snapshots
   - Point-in-time recovery
   - Retention policy: 7 days for development, 30 days for staging, 90 days for production

3. **Performance and Monitoring**:
   - Enable Enhanced Monitoring
   - Configure Performance Insights
   - Set up CloudWatch alarms for performance metrics

#### Database Security

1. **Access Control**:
   - Use IAM authentication where possible
   - Implement strict security groups
   - Encrypt data at rest and in transit

2. **Connection Pooling**:
   - Implement PgBouncer for connection pooling
   - Configure pool sizes based on workloads

### 4. Object Storage

Configure S3 buckets for content storage:

#### Bucket Structure

1. **Service-Specific Buckets**:
   - `authorworks-content-{env}`: Book content storage
   - `authorworks-media-{env}`: Media files (images, audio, video)
   - `authorworks-exports-{env}`: Exported content
   - `authorworks-backups-{env}`: Database backups
   - `authorworks-logs-{env}`: Application logs

2. **Lifecycle Policies**:
   - Transition infrequent access to cheaper storage classes
   - Set appropriate retention periods
   - Configure versioning for content

3. **Security Configuration**:
   - Encrypt buckets with KMS keys
   - Implement strict bucket policies
   - Enable access logging

### 5. Service Mesh

Implement Linkerd as the service mesh solution:

#### Mesh Configuration

1. **Core Features**:
   - Mutual TLS for service-to-service communication
   - Traffic management (retries, timeouts, circuit breaking)
   - Service discovery and load balancing
   - Observability (metrics, tracing, logging)

2. **Deployment Strategy**:
   - Install using Helm charts
   - Configure per-namespace policies
   - Implement automatic proxy injection

#### Implementation Example

```yaml
# linkerd-config.yaml
apiVersion: install.linkerd.io/v1alpha1
kind: Values
metadata:
  name: linkerd-values
spec:
  global:
    identityTrustAnchorsPEM: |
      # Trust anchor certificate
    clusterDomain: cluster.local
  identity:
    issuer:
      scheme: kubernetes.io/tls
  proxy:
    resources:
      cpu:
        limit: "1"
        request: "100m"
      memory:
        limit: "250Mi"
        request: "20Mi"
  proxyInit:
    resources:
      cpu:
        limit: "100m"
        request: "10m"
      memory:
        limit: "50Mi"
        request: "10Mi"
```

### 6. Observability Stack

Implement a comprehensive observability solution:

#### Components

1. **Monitoring**:
   - Prometheus for metrics collection
   - Grafana for dashboards and visualization
   - AlertManager for alerting

2. **Logging**:
   - Loki for log aggregation
   - Promtail for log collection
   - Configure log retention and indexing

3. **Distributed Tracing**:
   - Jaeger for trace collection and visualization
   - OpenTelemetry for instrumentation
   - Sampling configuration for production

#### Default Monitoring Configuration

```yaml
# prometheus-config.yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yaml"

scrape_configs:
  - job_name: 'kubernetes-pods'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2
        target_label: __address__
      - action: labelmap
        regex: __meta_kubernetes_pod_label_(.+)
      - source_labels: [__meta_kubernetes_namespace]
        action: replace
        target_label: kubernetes_namespace
      - source_labels: [__meta_kubernetes_pod_name]
        action: replace
        target_label: kubernetes_pod_name
```

#### Default Dashboards

Create default dashboards for:
1. Service health and performance
2. Database performance
3. API gateway metrics
4. Business metrics (users, content, subscriptions)
5. System resource utilization

### 7. CI/CD Pipeline Infrastructure

Set up infrastructure for continuous integration and deployment:

#### Build Environment

1. **GitHub Actions Runners**:
   - Self-hosted runners for performance
   - Runner isolation for security
   - Autoscaling based on demand

2. **Artifact Repository**:
   - ECR for container images
   - S3 for build artifacts
   - Versioning and tagging strategy

#### Deployment Pipeline

1. **ArgoCD Setup**:
   - GitOps-based deployment
   - Application of environments (dev, staging, production)
   - Sync policies and automation

2. **Canary Deployment**:
   - Traffic splitting capabilities
   - Automated rollback on failure
   - Progressive delivery configuration

### 8. Security Infrastructure

Implement security infrastructure for the platform:

#### Secret Management

1. **AWS Secrets Manager**:
   - Store database credentials
   - API keys and tokens
   - External service credentials

2. **Kubernetes Secrets**:
   - Service-to-service authentication
   - TLS certificates
   - Configuration secrets

#### Access Control

1. **IAM Policies**:
   - Least privilege principle
   - Service roles with minimal permissions
   - Developer roles for different environments

2. **RBAC Configuration**:
   - Namespace-based access control
   - Service account permissions
   - Admin and developer roles

## Implementation Steps

1. Create Terraform modules for core infrastructure
2. Set up VPC, networking, and security groups
3. Deploy EKS clusters for development environment
4. Configure databases and object storage
5. Install service mesh and observability stack
6. Set up CI/CD pipeline infrastructure
7. Implement security controls and access management
8. Repeat for staging and production with appropriate configurations

## Technical Decisions

### Why Amazon EKS?

Amazon EKS was chosen over self-managed Kubernetes or other managed services because:
- Managed control plane reduces operational overhead
- Native integration with AWS services
- Strong security posture with automatic upgrades
- Supports both EC2 and Fargate deployment options
- Enterprise-grade SLA for production workloads

### Why Linkerd over Istio?

Linkerd was selected as the service mesh solution because:
- Lightweight with minimal overhead
- Simple installation and configuration
- Strong performance characteristics
- Rust-based data plane aligns with our technology stack
- Excellent observability features

## Success Criteria

The infrastructure setup will be considered successfully implemented when:

1. All environments can be created and destroyed using Terraform
2. Kubernetes clusters are properly configured with all required components
3. Databases and storage are provisioned with appropriate security
4. Observability stack provides comprehensive monitoring
5. CI/CD pipeline can deploy to all environments
6. Security controls are validated through scanning and testing 