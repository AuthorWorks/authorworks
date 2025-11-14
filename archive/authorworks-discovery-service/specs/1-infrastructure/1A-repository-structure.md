# Technical Specification: 1A - Repository Structure and Organization

## Overview

This specification details the repository structure for the AuthorWorks platform. It establishes a multi-repo architecture with consistent standards and shared libraries.

## Objectives

- Establish a clean separation of concerns through microservices
- Enable parallel development across multiple teams
- Maintain consistent coding standards and practices
- Facilitate code reuse through shared libraries
- Streamline CI/CD processes

## Requirements

### 1. GitHub Organization

1. Create a GitHub organization named `authorworks`
2. Configure organization settings:
   - Enable two-factor authentication requirement
   - Set up default repository permissions (read for members)
   - Configure default branch protection rules
   - Create development, admin, and security teams

### 2. Repository Structure

Initialize the following repositories with standardized structure:

| Repository | Purpose | Technology | Dependencies |
|------------|---------|------------|--------------|
| `authorworks-ui-shell` | UI shell application | Dioxus | authorworks-shared |
| `authorworks-user-service` | User management service | Axum, PostgreSQL | authorworks-shared |
| `authorworks-content` | Content generation service | Axum, PostgreSQL | authorworks-shared |
| `authorworks-editor` | Editor service | Axum, Dioxus, Rustpad, Plate.js | authorworks-shared |
| `authorworks-storage` | Content storage service | Axum, S3 | authorworks-shared |
| `authorworks-messaging` | Matrix gateway service | Axum, Matrix SDK | authorworks-shared |
| `authorworks-graphics` | Text-to-graphic novel service | Axum, image processing | authorworks-shared |
| `authorworks-audio` | Text-to-audio service | Axum, audio processing | authorworks-shared |
| `authorworks-video` | Text-to-video service | Axum, video processing | authorworks-shared |
| `authorworks-payments` | Subscription service | Axum, PostgreSQL, Stripe | authorworks-shared |
| `authorworks-discovery` | Content discovery API | Axum, PostgreSQL, search | authorworks-shared |
| `authorworks-gateway` | API gateway service | Axum | authorworks-shared |
| `authorworks-shared` | Shared libraries | Rust | None |
| `authorworks-infra` | Infrastructure as code | Terraform, Kubernetes | None |
| `authorworks-docs` | Documentation | Markdown, MkDocs | None |

### 3. Standard Repository Components

Each service repository must contain:

```
repository/
├── .github/
│   └── workflows/
│       ├── ci.yml          # Continuous integration
│       ├── cd.yml          # Continuous deployment
│       └── security.yml    # Security scans
├── Dockerfile              # Container definition
├── docker-compose.yml      # Local development setup
├── Cargo.toml              # Dependencies and metadata
├── README.md               # Service documentation
├── docs/                   # Extended documentation
│   ├── setup.md
│   └── api.md
├── migrations/             # Database migrations (if applicable)
│   └── ...
├── src/
│   ├── main.rs             # Entry point
│   ├── config.rs           # Configuration
│   ├── error.rs            # Error handling
│   ├── api/                # API endpoints
│   ├── domain/             # Domain logic
│   ├── infrastructure/     # External services
│   └── repository/         # Data access
└── tests/
    ├── integration/        # Integration tests
    └── unit/               # Unit tests
```

### 4. Shared Library Structure

The `authorworks-shared` repository must contain:

```
authorworks-shared/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs                 # Library entry point
│   ├── models/                # Shared data models
│   │   ├── user.rs
│   │   ├── content.rs
│   │   └── ...
│   ├── auth/                  # Authentication utilities
│   │   ├── jwt.rs
│   │   ├── rbac.rs
│   │   └── ...
│   ├── validation/            # Input validation
│   ├── errors/                # Error types and handling
│   ├── telemetry/             # Logging and metrics
│   │   ├── logging.rs
│   │   ├── tracing.rs
│   │   └── metrics.rs
│   ├── api/                   # API utilities
│   │   ├── pagination.rs
│   │   ├── response.rs
│   │   └── ...
│   └── testing/               # Test utilities
└── tests/
```

### 5. Branch and Commit Strategy

- **Main branch**: `main` - stable, deployable code
- **Development branch**: `dev` - integration branch
- **Feature branches**: `feature/{feature-name}` - for new features
- **Release branches**: `release/{version}` - for release preparation
- **Hotfix branches**: `hotfix/{issue}` - for production fixes

Commits must follow the conventional commits specification:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting changes
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Build process or tool changes

### 6. Documentation Requirements

Each repository must include:

1. Comprehensive README.md with:
   - Service overview and purpose
   - Local development setup instructions
   - Testing procedures
   - Configuration options
   - API overview (if applicable)

2. API documentation:
   - OpenAPI specifications for HTTP APIs
   - Interface documentation for library functions
   - Authentication requirements
   - Examples of common operations

### 7. CI/CD Pipeline Configuration

Standard CI/CD workflows to include:

1. **Build and Test**:
   - Compile code
   - Run unit tests
   - Run integration tests
   - Check code coverage
   - Run linting and formatting checks

2. **Security Checks**:
   - Dependency vulnerability scanning
   - SAST (Static Application Security Testing)
   - License compliance
   - Secret scanning

3. **Deployment**:
   - Build container image
   - Push to container registry
   - Deploy to appropriate environment
   - Run smoke tests
   - Enable monitoring

## Implementation Steps

1. Create GitHub organization
2. Initialize repository templates
3. Create shared library repository
4. Establish CI/CD template workflows
5. Document repository structure and guidelines
6. Set up branch protection and code review requirements
7. Create initial issue templates and project boards

## Technical Decisions

### Why multi-repo over monorepo?

Multi-repo architecture was selected to:
- Enable independent service deployments
- Allow service-specific dependency management
- Support team autonomy
- Simplify permissions management
- Reduce build time for individual services

### Why Rust for all services?

Rust was chosen for:
- Memory safety without garbage collection
- Performance comparable to C/C++
- Strong type system and compile-time checks
- Growing ecosystem for web services
- Cross-platform compatibility

## Success Criteria

The repository structure will be considered successfully implemented when:

1. All repositories are created with standardized structure
2. CI/CD pipelines are functioning for each repository
3. Teams can work independently on separate services
4. Shared libraries can be consumed by all services
5. Documentation standards are established and followed 