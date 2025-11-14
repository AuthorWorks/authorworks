# Technical Specification: 1C - Service Interface Definitions

## Overview

This specification details the API contracts and data models that will be shared across all AuthorWorks services. It establishes clear boundaries and consistent interfaces for inter-service communication.

## Objectives

- Define clear API contracts between services
- Establish shared data models and validation rules
- Implement consistent error handling across services
- Create version control strategy for APIs
- Generate comprehensive API documentation

## Requirements

### 1. OpenAPI Specifications

Create OpenAPI 3.1 specifications for all service APIs with the following requirements:

#### Specification Format

Each service must provide an OpenAPI 3.1 specification in YAML format at the path `/docs/openapi.yaml` within its repository. The specification must:

- Include comprehensive descriptions for all endpoints
- Document all request parameters and response structures
- Define security schemes
- Include examples for all operations
- Specify response codes and error formats

#### Minimal OpenAPI Template

```yaml
openapi: 3.1.0
info:
  title: AuthorWorks [Service Name] API
  version: 1.0.0
  description: |
    API for the [Service Name] microservice of the AuthorWorks platform.
  contact:
    name: AuthorWorks Engineering
    email: engineering@authorworks.io
    url: https://github.com/authorworks-io
servers:
  - url: https://{environment}.api.authorworks.io/v1
    variables:
      environment:
        enum:
          - dev
          - staging
          - prod
        default: dev
paths:
  # Paths defined here
components:
  schemas:
    # Shared data models
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
security:
  - bearerAuth: []
```

### 2. Shared Data Models

Define core data models to be used across services, implemented in the `authorworks-shared` repository:

#### Core Models

1. **User Model**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct User {
       pub id: Uuid,
       pub email: String,
       pub display_name: Option<String>,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
       pub status: UserStatus,
       pub roles: Vec<Role>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum UserStatus {
       Active,
       Inactive,
       Suspended,
       Unverified,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum Role {
       User,
       Creator,
       Admin,
       IndustryProfessional,
   }
   ```

2. **Content Model**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Book {
       pub id: Uuid,
       pub user_id: Uuid,
       pub title: String,
       pub description: Option<String>,
       pub genre: Option<String>,
       pub tags: Vec<String>,
       pub cover_image_url: Option<String>,
       pub status: ContentStatus,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
       pub published_at: Option<DateTime<Utc>>,
       pub metadata: Option<BookMetadata>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Chapter {
       pub id: Uuid,
       pub book_id: Uuid,
       pub title: String,
       pub sequence_number: i32,
       pub summary: Option<String>,
       pub status: ContentStatus,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Scene {
       pub id: Uuid,
       pub chapter_id: Uuid,
       pub title: Option<String>,
       pub sequence_number: i32,
       pub content: Option<String>,
       pub summary: Option<String>,
       pub status: ContentStatus,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ContentStatus {
       Draft,
       InProgress,
       Review,
       Published,
       Archived,
   }
   ```

3. **Subscription Model**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Subscription {
       pub id: Uuid,
       pub subscriber_id: Uuid,
       pub creator_id: Uuid,
       pub tier_id: Uuid,
       pub stripe_subscription_id: String,
       pub status: SubscriptionStatus,
       pub current_period_start: DateTime<Utc>,
       pub current_period_end: DateTime<Utc>,
       pub cancel_at_period_end: bool,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SubscriptionTier {
       pub id: Uuid,
       pub creator_id: Uuid,
       pub name: String,
       pub description: Option<String>,
       pub price_cents: i32,
       pub currency: String,
       pub billing_interval: BillingInterval,
       pub features: Option<Vec<String>>,
       pub active: bool,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum SubscriptionStatus {
       Active,
       PastDue,
       Unpaid,
       Canceled,
       Incomplete,
       IncompleteExpired,
       Trialing,
       Paused,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum BillingInterval {
       Monthly,
       Quarterly,
       Biannual,
       Annual,
   }
   ```

4. **API Common Models**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PaginatedResponse<T> {
       pub items: Vec<T>,
       pub pagination: Pagination,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Pagination {
       pub total: u64,
       pub page: u32,
       pub page_size: u32,
       pub pages: u32,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ApiError {
       pub code: String,
       pub message: String,
       pub details: Option<serde_json::Value>,
   }
   ```

#### Model Validation

Implement validation traits for all models using the validator crate:

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Book {
    pub id: Uuid,
    pub user_id: Uuid,
    
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(max = 5000))]
    pub description: Option<String>,
    
    #[validate(length(max = 100))]
    pub genre: Option<String>,
    
    #[validate(custom = "validate_tags")]
    pub tags: Vec<String>,
    
    #[validate(url)]
    pub cover_image_url: Option<String>,
    
    pub status: ContentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub metadata: Option<BookMetadata>,
}

fn validate_tags(tags: &[String]) -> Result<(), ValidationError> {
    if tags.len() > 10 {
        return Err(ValidationError::new("too_many_tags"));
    }
    
    for tag in tags {
        if tag.len() > 50 {
            return Err(ValidationError::new("tag_too_long"));
        }
    }
    
    Ok(())
}
```

### 3. API Standardization

Define standard patterns for all service APIs:

#### RESTful Endpoint Patterns

All endpoints must follow RESTful principles:

| HTTP Method | Path Pattern | Purpose | Example |
|-------------|--------------|---------|---------|
| GET | /v1/{resource} | List resources | GET /v1/books |
| GET | /v1/{resource}/{id} | Get resource by ID | GET /v1/books/123 |
| POST | /v1/{resource} | Create resource | POST /v1/books |
| PUT | /v1/{resource}/{id} | Replace resource | PUT /v1/books/123 |
| PATCH | /v1/{resource}/{id} | Partial update | PATCH /v1/books/123 |
| DELETE | /v1/{resource}/{id} | Delete resource | DELETE /v1/books/123 |
| GET | /v1/{resource}/{id}/{subresource} | List subresources | GET /v1/books/123/chapters |
| POST | /v1/{resource}/{id}/{subresource} | Create subresource | POST /v1/books/123/chapters |

#### Query Parameter Standards

Standardize common query parameters:

- `page`: Page number for pagination (default: 1)
- `page_size`: Number of items per page (default: 20, max: 100)
- `sort`: Field to sort by (format: `field` or `-field` for descending)
- `fields`: Comma-separated list of fields to include in response
- `q`: Search query parameter
- `filter_{field}`: Filter by specific field value

#### Response Format

All API responses must follow consistent formats:

1. **Success Response**:
   ```json
   {
     "data": {
       // Resource data
     }
   }
   ```

2. **List Response**:
   ```json
   {
     "data": [
       // Array of resources
     ],
     "pagination": {
       "total": 100,
       "page": 1,
       "page_size": 20,
       "pages": 5
     }
   }
   ```

3. **Error Response**:
   ```json
   {
     "error": {
       "code": "ERROR_CODE",
       "message": "Human-readable message",
       "details": {
         // Optional additional error details
       }
     }
   }
   ```

#### Status Codes

Use standard HTTP status codes consistently:

- `200 OK`: Successful request
- `201 Created`: Resource created
- `204 No Content`: Successful request with no response body
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Missing authentication
- `403 Forbidden`: Authenticated but not authorized
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate)
- `422 Unprocessable Entity`: Validation errors
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error

### 4. Versioning Strategy

Implement API versioning using the following approach:

#### URL Path Versioning

Include version in the URL path: `/v1/{resource}`

Benefits:
- Explicit and visible versioning
- Clear separation between versions
- Easy routing in API gateway

#### Version Compatibility Rules

1. Within a major version:
   - Can add new optional fields
   - Can add new endpoints
   - Cannot remove or rename fields
   - Cannot change field types
   - Cannot remove endpoints

2. For breaking changes:
   - Increment major version
   - Maintain previous versions for transition period
   - Document deprecation schedule

#### Deprecation Process

1. Mark endpoints as deprecated using the `Sunset` header
2. Include deprecation notice in API documentation
3. Provide migration guide to new endpoints
4. Set deprecation timeline (minimum 3 months notice)
5. Monitor usage of deprecated endpoints

### 5. API Documentation Generation

Implement automated API documentation generation:

#### Documentation Tools

1. Generate OpenAPI UI using Swagger UI:
   - Expose at `/docs` endpoint on each service
   - Include examples for all operations
   - Add authentication flow documentation

2. Create service-to-service API documentation:
   - Document internal APIs
   - Specify authentication requirements
   - Include error handling guidelines

#### Documentation Build Process

1. Generate documentation as part of CI/CD pipeline
2. Validate OpenAPI specifications for correctness
3. Publish documentation to central repository
4. Version documentation to match API versions

## Implementation Steps

1. Create shared data models in `authorworks-shared`
2. Implement validation traits for all models
3. Define OpenAPI templates for each service
4. Create documentation generation pipeline
5. Implement API versioning strategy
6. Develop contract testing suite

## Technical Decisions

### Why OpenAPI 3.1?

OpenAPI 3.1 was chosen for API specification because:
- It's the industry standard for REST API documentation
- Supports JSON Schema 2020-12 for more accurate type definitions
- Provides better tooling support for validation and code generation
- Allows for more detailed documentation with markdown

### Why URL Path Versioning?

URL path versioning was selected over header or parameter versioning because:
- It's more visible and explicit to API consumers
- Simplifies routing in the API gateway
- Makes debugging easier as version is visible in logs
- Allows clients to use multiple versions simultaneously

## Success Criteria

The service interface definitions will be considered successfully implemented when:

1. All services have complete OpenAPI specifications
2. Shared data models are implemented and validated
3. APIs follow standardized RESTful patterns
4. Documentation is generated and published
5. Contract tests verify API compliance 