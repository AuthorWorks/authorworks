# AuthorWorks Deployment Strategy

## Overview

This document outlines the deployment strategy for AuthorWorks, focusing on cost-effective self-hosting for backend services while leveraging cloud resources only for customer-facing components. This approach maximizes control and minimizes ongoing costs while ensuring reliable delivery to end users.

## Infrastructure Resources

### Self-Hosted Resources

#### Primary Home Server
- **Hardware**: Ubuntu server with 55GB RAM, 6TB external storage
- **Network**: Fiber internet connection with static IP or dynamic DNS solution
- **Purpose**: Hosting core microservices, databases, and processing workloads

#### Raspberry Pi Cluster
- **Hardware**: 30-core Raspberry Pi cluster (each with ~1GB RAM, 16-64GB storage)
- **Management**: Coolify for container orchestration
- **Purpose**: Distributed processing, monitoring, and auxiliary services

### Cloud Resources

- **AWS S3**: Static file storage for customer-facing assets
- **AWS CloudFront**: CDN for global content delivery
- **AWS Route53**: DNS management for the public domain

## Service Deployment Strategy

### Core Services Deployment (Self-Hosted)

| Service | Deployment | Hardware | Resources | Notes |
|---------|------------|----------|-----------|-------|
| API Gateway | Docker on Home Server | Ubuntu | 200-400MB RAM | Traefik integration |
| User Service | Docker on Home Server | Ubuntu | 200-400MB RAM | Auth integration with Authelia |
| Content Service | Docker on Home Server | Ubuntu | 500MB-1GB RAM | Postgres backend |
| Subscription Service | Docker on Home Server | Ubuntu | 300-500MB RAM | Local payment processing |
| Storage Service | Docker on Home Server | Ubuntu | 1-2GB RAM | MinIO integration |
| Editor Service | Docker on Home Server | Ubuntu | 1-2GB RAM | Collaborative editing |
| Messaging Service | Docker on Home Server | Ubuntu | 300-500MB RAM | Uses existing Redis |
| Discovery Service | Docker on Home Server | Ubuntu | 1-2GB RAM | Vector DB integration |
| Graphics Service | Docker on Home Server | Ubuntu | 2-4GB RAM | AI model integration |
| Audio Service | Docker on Home Server | Ubuntu | 1-2GB RAM | Audio processing |
| Video Service | Docker on Home Server | Ubuntu | 2-4GB RAM | Video processing |

### AI Processing Services (Self-Hosted)

| Service | Deployment | Hardware | Resources | Notes |
|---------|------------|----------|-----------|-------|
| Text Generation | Docker on Home Server | Ubuntu | 4-6GB RAM | Ollama integration |
| Image Generation | Docker on Home Server | Ubuntu | 3-5GB RAM | Local StableDiffusion |
| Audio Synthesis | Docker on Home Server | Ubuntu | 2-3GB RAM | Local TTS model |

### Auxiliary Services (Raspberry Pi Cluster)

| Service | Deployment | Hardware | Resources | Notes |
|---------|------------|----------|-----------|-------|
| Monitoring | Docker on Pi | Raspberry Pi | 500MB RAM | Prometheus/Grafana |
| Log Management | Docker on Pi | Raspberry Pi | 500MB RAM | Loki integration |
| Background Jobs | Docker on Pi | Raspberry Pi | 500MB RAM | Task distribution |
| Backup System | Docker on Pi | Raspberry Pi | 300MB RAM | Syncthing integration |
| Health Checks | Docker on Pi | Raspberry Pi | 200MB RAM | Uptime Kuma |

### Customer-Facing Components (AWS)

| Component | Service | Purpose | Notes |
|-----------|---------|---------|-------|
| Web Application | S3 + CloudFront | Serve UI assets | Static site with SPA |
| API Endpoint | Route53 + Home Server | Public API access | Secure tunnel to home |
| Media Assets | S3 + CloudFront | Serve user media | Optimized delivery |

## Networking Configuration

### Inbound Traffic Flow

1. User requests hit CloudFront distribution
2. Static assets served directly from S3
3. API requests forwarded to home server via secure tunnel
4. Traefik routes requests to appropriate microservices

### Security Measures

1. **Cloudflare Tunnel**: Secure inbound connection without exposing home IP
2. **OAuth/JWT**: Authentication via Authelia integrated with User Service
3. **Rate Limiting**: Implemented at both CloudFront and API Gateway levels
4. **WAF Rules**: Basic protection at CloudFront level
5. **Internal Network Segmentation**: Docker networks for service isolation

## Database Strategy

### Primary Data Stores

| Database | Deployment | Purpose | Backup Strategy |
|----------|------------|---------|----------------|
| PostgreSQL | Docker on Home Server | Primary relational data | Daily snapshots to external drive + S3 |
| Redis | Docker on Home Server | Caching, queues | Persistence to disk |
| MinIO | Docker on Home Server | Object storage | Replication to external drive |
| Vector | Docker on Home Server | Search indexing | Periodic rebuilds from primary data |

### Data Backup Strategy

1. **Local Backups**: Daily snapshots to external drive
2. **Off-site Backups**: Weekly encrypted backups to S3 Glacier
3. **Database Dumps**: Hourly logical backups for point-in-time recovery
4. **Syncthing**: Real-time sync between critical systems

## Scaling Strategy

### Vertical Scaling

1. **RAM Upgrades**: Identify memory bottlenecks and upgrade as needed
2. **Storage Expansion**: Add additional external drives as needed
3. **CPU Upgrades**: Monitor for CPU-bound workloads

### Horizontal Scaling

1. **Raspberry Pi Expansion**: Add nodes to cluster for specific workloads
2. **Service Replication**: Run multiple instances behind load balancer
3. **Database Read Replicas**: Deploy read-only copies for heavy read workloads

### Cloud Overflow

1. **Elastic Processing**: Burst to cloud for heavy processing tasks
2. **CDN Scaling**: CloudFront handles traffic spikes for static content
3. **Media Processing**: Option to offload to cloud for intensive tasks

## Development Workflow

### Local Development

1. Docker Compose for local service development
2. MinIO for S3-compatible local storage
3. LocalStack for AWS service emulation where needed

### CI/CD Pipeline

1. GitHub Actions for automated testing
2. Automated builds of Docker images
3. Deployment scripts for home server updates
4. Blue/Green deployment strategy for zero-downtime updates

## Monitoring and Observability

### Self-Hosted Monitoring Stack

1. **Prometheus**: Metrics collection from all services
2. **Grafana**: Visualization and dashboards
3. **Loki**: Log aggregation and search
4. **Uptime Kuma**: External availability monitoring
5. **Node-exporter**: Hardware-level metrics

### Alerting

1. Matrix notifications for critical alerts
2. Email notifications for daily summaries
3. SMS alerts for severe outages

## Disaster Recovery

### Recovery Procedures

1. **Service Failure**: Automatic restart via Docker
2. **Data Corruption**: Restore from latest backup
3. **Hardware Failure**: Migration to backup hardware
4. **Network Outage**: Temporary redirect to minimal cloud backup

### Recovery Time Objectives

1. **Service Disruption**: < 5 minutes
2. **Partial Data Loss**: < 1 hour
3. **Complete System Failure**: < 24 hours

## Cost Analysis

### One-time Costs

1. **Home Server**: Already owned
2. **Raspberry Pi Cluster**: Already owned
3. **External Storage**: Already owned

### Recurring Costs

1. **AWS S3**: ~$0.023/GB for storage
2. **AWS CloudFront**: ~$0.085/GB for transfer
3. **AWS Route53**: $0.50/month per hosted zone
4. **Electricity**: ~$20-30/month for all hardware
5. **Domain Name**: ~$10-15/year

### Cost Comparison

| Scenario | AWS Cloud Only | Self-Hosted + AWS | Savings |
|----------|----------------|-------------------|---------|
| Development | $200-300/month | $20-30/month | 85-90% |
| 100 Users | $300-500/month | $50-100/month | 80-85% |
| 1000 Users | $1000-1500/month | $200-300/month | 75-80% |

## Implementation Plan

### Phase 1: Core Infrastructure (Weeks 1-2)

1. Set up Traefik as API Gateway on home server
2. Configure S3 buckets and CloudFront distribution
3. Establish secure tunneling from CloudFront to home server
4. Deploy PostgreSQL, Redis, and MinIO containers
5. Configure backup systems

### Phase 2: Core Services (Weeks 3-6)

1. Deploy User Service with Authelia integration
2. Deploy Content Service with PostgreSQL backend
3. Deploy Storage Service with MinIO integration
4. Deploy basic UI to S3/CloudFront
5. Set up monitoring stack on Raspberry Pi cluster

### Phase 3: Advanced Services (Weeks 7-12)

1. Deploy Editor Service with collaborative editing
2. Deploy AI services with Ollama integration
3. Deploy Audio/Video/Graphics processing services
4. Implement background processing on Raspberry Pi cluster
5. Complete full system testing and optimization

## Conclusion

This deployment strategy leverages existing self-hosted infrastructure while minimizing cloud costs. By keeping processing and data storage on-premises and using cloud services only for customer-facing components, we achieve a cost-effective solution without sacrificing reliability or performance.

The approach provides significant cost savings compared to a cloud-only deployment while maintaining control over sensitive data and processing workflows. As user numbers grow, the architecture can scale through targeted hardware upgrades and selective use of cloud resources for overflow capacity. 