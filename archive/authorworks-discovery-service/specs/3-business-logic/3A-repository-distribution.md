# Repository Distribution for AuthorWorks Specifications

This document outlines how AuthorWorks specifications should be distributed across repositories in alignment with our microservices architecture.

## 1. Repository Structure Overview

AuthorWorks follows a multi-repository (poly-repo) approach for its microservices architecture. Each service has its own dedicated repository, allowing teams to work independently while maintaining clear boundaries between services.

### 1.1 Repository Organization

- **Base Directory**: `~/git/aw/` serves as the base directory for all AuthorWorks repositories
- **Naming Convention**: All repositories follow the `authorworks-[component]` naming pattern
- **Repository Structure**: Each repository has a consistent internal structure with `src/`, `docs/`, and `specs/` directories

### 1.2 Core Repositories

- **authorworks** - Central repository for platform-wide documentation, infrastructure code, and deployment orchestration
- **authorworks-content-service** - Content management service
- **authorworks-user-service** - User authentication and management service
- **authorworks-subscription-service** - Subscription and payment handling service
- **authorworks-storage-service** - File storage and retrieval service
- **authorworks-editor-service** - Text and media editing service
- **authorworks-messaging-service** - Internal and user notifications service
- **authorworks-discovery-service** - Content discovery and search service
- **authorworks-graphics-service** - Image and graphics processing service
- **authorworks-audio-service** - Audio processing and management service
- **authorworks-video-service** - Video processing and management service
- **authorworks-ui-shell** - Main UI framework and shell
- **authorworks-ui** - UI components and libraries
- **authorworks-docs** - Public documentation and API references

## 2. Specification Distribution

Specifications should be distributed across repositories to ensure teams have access to relevant documentation while maintaining a single source of truth.

### 2.1 Infrastructure Specifications (1-infrastructure)

Location: **authorworks** repository

| Specification | Purpose | Location |
|---------------|---------|----------|
| 1A-api-gateway.md | API Gateway configuration and routing | `/specs/infrastructure/` |
| 1B-service-discovery.md | Service discovery and registration | `/specs/infrastructure/` |
| 1C-monitoring.md | Monitoring and observability | `/specs/infrastructure/` |
| 1D-deployment.md | Deployment pipelines and strategies | `/specs/infrastructure/` |

### 2.2 Service Specifications (2-services)

Each service specification should be stored in its respective service repository, with a synchronized copy in the platform repository for reference.

| Service Spec | Primary Location | Reference Copy |
|--------------|------------------|---------------|
| 2A-api-gateway.md | authorworks/specs/gateway/api-gateway-spec.md | N/A |
| 2B-user-service.md | authorworks-user-service/specs/service-spec.md | authorworks/specs/services/user-service.md |
| 2C-content-service.md | authorworks-content-service/specs/service-spec.md | authorworks/specs/services/content-service.md |
| 2D-subscription-service.md | authorworks-subscription-service/specs/service-spec.md | authorworks/specs/services/subscription-service.md |
| 2E-storage-service.md | authorworks-storage-service/specs/service-spec.md | authorworks/specs/services/storage-service.md |
| 2F-editor-service.md | authorworks-editor-service/specs/service-spec.md | authorworks/specs/services/editor-service.md |
| 2G-messaging-service.md | authorworks-messaging-service/specs/service-spec.md | authorworks/specs/services/messaging-service.md |
| 2H-discovery-service.md | authorworks-discovery-service/specs/service-spec.md | authorworks/specs/services/discovery-service.md |
| 2I-graphics-service.md | authorworks-graphics-service/specs/service-spec.md | authorworks/specs/services/graphics-service.md |
| 2J-audio-service.md | authorworks-audio-service/specs/service-spec.md | authorworks/specs/services/audio-service.md |
| 2K-video-service.md | authorworks-video-service/specs/service-spec.md | authorworks/specs/services/video-service.md |
| 2L-ui-shell-service.md | authorworks-ui-shell/specs/service-spec.md | authorworks/specs/services/ui-shell-service.md |

### 2.3 Business Logic Specifications (3-business-logic)

Business logic specifications should be distributed based on which service implements the logic.

| Business Logic Spec | Location |
|---------------------|----------|
| 3A-repository-distribution.md | authorworks/specs/business-logic/ |
| 3B-publishing-workflow.md | authorworks/specs/business-logic/ |
| 3C-subscription-lifecycle.md | authorworks-subscription-service/specs/business-logic/ |
| 3D-content-workflows.md | authorworks-content-service/specs/business-logic/ |
| 3E-user-permissions.md | authorworks-user-service/specs/business-logic/ |

### 2.4 UI Specifications (4-ui)

UI specifications should be stored in the UI repository, with appropriate cross-references to service APIs they consume.

| UI Spec | Location |
|---------|----------|
| 4A-component-library.md | authorworks-ui/specs/component-library.md |
| 4B-editor-interface.md | authorworks-ui/specs/editor-interface.md |
| 4C-dashboard-layouts.md | authorworks-ui/specs/dashboard-layouts.md |
| 4D-responsive-design.md | authorworks-ui/specs/responsive-design.md |

## 3. Specification Synchronization

To maintain consistency while allowing service teams to evolve their specifications, we establish the following synchronization patterns:

### 3.1 Synchronization Process

1. **Central Repository as Source of Truth**: The main AuthorWorks repository (`~/git/authorworks/`) acts as the source of truth for all specifications
2. **Bi-directional Synchronization**: Changes are synchronized in both directions:
   - From the main repository to service repositories
   - From service repositories back to the main repository and then to other services
3. **Automated Synchronization**: GitHub Actions workflows automatically handle the synchronization process
4. **Pull Request Reviews**: Changes to specifications require cross-team reviews before merging to the main branch
5. **Version Tagging**: Major specification changes are tagged with versions

### 3.2 Synchronization Scripts and Workflows

The repository synchronization is managed through:

1. **repo_setup.sh**: Located in the `scripts/` directory, this script handles repository creation, cloning, and initial synchronization
2. **Main Repository GitHub Workflow**: Triggered when changes are made to specifications in the main repository, syncs to service repositories
3. **Service Repository GitHub Workflows**: Triggered when changes are made to specifications in service repositories, syncs back to the main repository

### 3.3 Synchronization Security

1. **GitHub Secret: SYNC_TOKEN**: A personal access token with write access to all repositories
2. **Restricted Permissions**: The token should have the minimum necessary permissions for the sync operations
3. **Isolated Workflow Jobs**: Each synchronization operation runs in its own isolated job

### 3.4 Conflict Resolution

When conflicts arise between specifications:

1. The main repository's specification takes precedence over service-specific modifications
2. Service-specific implementation details should be added as extensions, not modifications
3. Cross-cutting concerns are resolved via architecture review board

## 4. Working with Multiple Repositories

Guidelines for developers working with the multi-repository structure:

### 4.1 Local Development Setup

```bash
# Clone the main repository
git clone https://github.com/authorworks/authorworks.git ~/git/authorworks

# Set up all service repositories
cd ~/git/authorworks
./scripts/repo_setup.sh sync
```

### 4.2 Making Changes to Specifications

1. **Make changes in the appropriate repository**:
   - For service-specific specifications: Edit in the service repository
   - For cross-cutting concerns: Edit in the main repository
2. **Push changes to the main branch**:
   - Service repository changes will be automatically synced to the main repository
   - Main repository changes will be automatically synced to service repositories
3. **Verify synchronization**:
   - Check the GitHub Actions workflow logs to ensure synchronization completed successfully

### 4.3 Service Development

1. Navigate to the specific service directory: `cd ~/git/aw/authorworks-[service-name]`
2. Reference specifications in the `specs/` directory
3. Implement features according to the specifications

## 5. Cross-Repository References

To maintain connections between distributed specifications:

1. **Unique Identifiers**: Each specification section has a unique ID (e.g., USER-SPEC-1.2)
2. **Hyperlink References**: Cross-repository references use standardized URLs
3. **Interface Contracts**: Service interfaces are versioned and referenced by version

Example reference format:
```
[User Authentication Flow](https://github.com/authorworks/authorworks-user-service/blob/main/specs/business-logic/authentication.md#USER-FLOW-1)
```

## 6. Implementation Steps

To transition to this repository distribution model:

1. Update the `repo_setup.sh` script to use the new base directory
2. Configure GitHub Actions workflow for automated synchronization
3. Migrate existing specifications to the main repository
4. Run the synchronization script to distribute specifications
5. Train teams on the new repository structure and workflow

## 7. Success Criteria

- All specifications have a clear home repository
- Specifications are successfully synchronized across repositories
- Teams can find and reference specifications easily
- Cross-service changes follow the established process
- Specification formats are consistent across repositories 