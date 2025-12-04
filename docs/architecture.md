# AuthorWorks Architecture

## System Overview

AuthorWorks is an AI-powered creative content platform built as a microservices architecture.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Load Balancer                                   │
│                    (Traefik / Nginx / AWS ALB)                              │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
┌─────────────────────────────────────┼───────────────────────────────────────┐
│                              API Gateway                                     │
│                              (Nginx)                                         │
└───┬─────────┬─────────┬─────────┬───┴───┬─────────┬─────────┬─────────┬─────┘
    │         │         │         │       │         │         │         │
┌───┴───┐ ┌───┴───┐ ┌───┴───┐ ┌───┴───┐ ┌─┴─────┐ ┌─┴─────┐ ┌─┴─────┐ ┌─┴─────┐
│ User  │ │Content│ │Storage│ │Editor │ │Subscr.│ │Messag.│ │Discov.│ │ Media │
│Service│ │Service│ │Service│ │Service│ │Service│ │Service│ │Service│ │Service│
└───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬───┘
    │         │         │         │         │         │         │         │
    └─────────┴─────────┴─────────┴────┬────┴─────────┴─────────┴─────────┘
                                       │
┌──────────────────────────────────────┼──────────────────────────────────────┐
│                           Infrastructure                                     │
│  ┌──────────┐  ┌─────────┐  ┌──────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │PostgreSQL│  │  Redis  │  │ RabbitMQ │  │Elasticsearch│  │  MinIO/S3   │ │
│  └──────────┘  └─────────┘  └──────────┘  └─────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Frontend** | Leptos (Rust/WASM) | Single-page application |
| **API Gateway** | Nginx | Routing, rate limiting, SSL |
| **Services** | Rust + Spin | WASM microservices |
| **Auth** | Logto | OAuth2/OIDC identity provider |
| **Database** | PostgreSQL | Primary data store |
| **Cache** | Redis | Caching, sessions, pub/sub |
| **Queue** | RabbitMQ | Async job processing |
| **Search** | Elasticsearch | Full-text search |
| **Storage** | MinIO/S3 | Object storage |
| **Observability** | Prometheus + Grafana + Loki | Monitoring & logging |

## Microservices

### User Service
- **Port**: 3101
- **Responsibilities**: Authentication, user profiles, preferences
- **Integrates with**: Logto, PostgreSQL, Redis

### Content Service
- **Port**: 3102
- **Responsibilities**: Story/book CRUD, AI content generation
- **Integrates with**: PostgreSQL, RabbitMQ, Anthropic API

### Storage Service
- **Port**: 3103
- **Responsibilities**: File uploads, downloads, presigned URLs
- **Integrates with**: MinIO/S3, Redis

### Editor Service
- **Port**: 3104
- **Responsibilities**: Editing sessions, collaborative editing
- **Integrates with**: PostgreSQL, Redis

### Subscription Service
- **Port**: 3105
- **Responsibilities**: Billing, payments, subscriptions
- **Integrates with**: PostgreSQL, Stripe

### Messaging Service
- **Port**: 3106
- **Responsibilities**: Real-time messaging, WebSocket
- **Integrates with**: Redis, RabbitMQ

### Discovery Service
- **Port**: 3107
- **Responsibilities**: Search, recommendations
- **Integrates with**: PostgreSQL, Elasticsearch, Redis

### Media Service
- **Port**: 3108
- **Responsibilities**: Audio/video/image processing
- **Integrates with**: MinIO/S3, RabbitMQ

## Background Workers

### Content Worker
- Processes AI content generation jobs
- Consumes from RabbitMQ `content_generation` queue
- Calls Anthropic/OpenAI APIs

### Media Worker
- Processes media transformation jobs
- Consumes from RabbitMQ `media_processing` queue
- Uses FFmpeg, ImageMagick

## Authentication Flow

```
┌──────────┐     ┌───────────┐     ┌──────────┐     ┌──────────────┐
│  Client  │────▶│API Gateway│────▶│  Logto   │────▶│ User Service │
└──────────┘     └───────────┘     └──────────┘     └──────────────┘
     │                                   │                  │
     │  1. Login request                 │                  │
     │──────────────────────────────────▶│                  │
     │                                   │                  │
     │  2. Redirect to Logto             │                  │
     │◀──────────────────────────────────│                  │
     │                                   │                  │
     │  3. User authenticates            │                  │
     │──────────────────────────────────▶│                  │
     │                                   │                  │
     │  4. Callback with code            │                  │
     │◀──────────────────────────────────│                  │
     │                                   │                  │
     │  5. Exchange code for tokens      │                  │
     │──────────────────────────────────▶│──────────────────▶│
     │                                   │                  │
     │  6. Return JWT tokens             │                  │
     │◀──────────────────────────────────│◀─────────────────│
```

## Data Flow

### Content Generation

```
┌────────┐    ┌─────────┐    ┌──────────┐    ┌────────────┐    ┌─────────┐
│ Client │───▶│ Content │───▶│ RabbitMQ │───▶│  Content   │───▶│Anthropic│
│        │    │ Service │    │          │    │  Worker    │    │   API   │
└────────┘    └─────────┘    └──────────┘    └────────────┘    └─────────┘
                  │                               │
                  │                               │
                  ▼                               ▼
            ┌──────────┐                   ┌──────────┐
            │PostgreSQL│◀──────────────────│PostgreSQL│
            └──────────┘                   └──────────┘
```

## Database Schema

### Core Tables

```sql
-- Users
users.users (id, email, username, display_name, ...)
users.profiles (user_id, bio, avatar_url, ...)
users.user_preferences (user_id, key, value)

-- Content
content.books (id, author_id, title, description, genre, ...)
content.chapters (id, book_id, title, content, chapter_number, ...)

-- Subscriptions
subscriptions.plans (id, name, price_cents, ...)
subscriptions.user_subscriptions (id, user_id, plan_id, status, ...)
```

## Security

### Network Security
- All inter-service communication on internal Docker network
- External traffic only through API Gateway
- TLS termination at load balancer

### Authentication
- Logto handles OAuth2/OIDC
- JWT tokens for API authentication
- Refresh token rotation

### Authorization
- Role-based access control (RBAC)
- Service-to-service auth via internal JWT

## Scalability

### Horizontal Scaling
- All services are stateless
- Can scale independently via Kubernetes HPA
- Redis for shared state

### Performance Optimizations
- WASM services have sub-millisecond cold starts
- Redis caching for frequent queries
- Elasticsearch for search offloading
- CDN for static assets

## Monitoring

### Metrics (Prometheus)
- Request rates, latencies, errors
- Resource utilization
- Custom business metrics

### Logging (Loki)
- Structured JSON logs
- Correlation IDs for tracing
- Log aggregation per service

### Dashboards (Grafana)
- Service health overview
- Error rates and alerts
- Resource utilization

