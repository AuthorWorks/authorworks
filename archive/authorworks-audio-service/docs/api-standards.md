# AuthorWorks API Standards

## Overview

This document defines the standardized approach to API design across all AuthorWorks microservices. Adhering to these standards ensures a consistent developer experience, simplifies integration, and reduces maintenance overhead.

## URL Structure and Versioning

### Base URL Format

All service APIs must follow this structure:
```
https://{service-name}.api.authorworks.local/v1/{resource}
```

For the self-hosted environment:
```
https://api.authorworks.local/v1/{service-name}/{resource}
```

### Versioning

- **All services MUST use the `/v1/` prefix in URL paths**
- Version numbers should use whole integers (v1, v2, v3)
- Breaking changes require a version increment
- Multiple API versions may be maintained simultaneously during transition periods
- Version information should NOT appear in HTTP headers or query parameters

### Endpoint Naming Conventions

- **Use pluralized resource names**: `/v1/books` not `/v1/book`
- **Use kebab-case for multi-word resources**: `/v1/publishing-rights`
- **Avoid verbs in URL paths** for CRUD operations, use appropriate HTTP methods
- **Use verb phrases for operations** that don't map cleanly to CRUD: `/v1/books/{id}/publish`

## HTTP Methods

| Method | Purpose | Response Code (Success) | Idempotent |
|--------|---------|-------------------------|------------|
| GET | Retrieve resources | 200 OK | Yes |
| POST | Create resources or trigger operations | 201 Created or 200 OK | No |
| PUT | Replace resources completely | 200 OK | Yes |
| PATCH | Update resources partially | 200 OK | No* |
| DELETE | Remove resources | 204 No Content | Yes |

*PATCH can be made idempotent with careful implementation

## Request Format

### Headers

| Header | Purpose | Required | Format |
|--------|---------|----------|--------|
| Authorization | Authentication | Yes* | `Bearer {token}` |
| Content-Type | Request content format | Yes (for requests with body) | `application/json` |
| Accept | Response format | No | `application/json` |
| X-Request-ID | Request tracking | No | UUID |
| X-Correlation-ID | Request chain tracking | No | UUID |

*Except for authentication endpoints

### Query Parameters

- **Filtering**: `?property=value&otherProperty=otherValue`
- **Sorting**: `?sort=propertyName:asc,otherProperty:desc`
- **Pagination**: `?page=1&per_page=20` or `?offset=0&limit=20`
- **Sparse fieldsets**: `?fields=prop1,prop2,prop3`
- **Embedding related resources**: `?embed=author,categories`

## Response Format

### Standard JSON Structure

All responses should follow a consistent structure:

```json
{
  "data": {
    // Primary response data (object or array)
  },
  "meta": {
    "request_id": "unique-request-identifier",
    "timestamp": "2023-06-15T12:34:56Z"
  },
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 342,
    "total_pages": 18
  },
  "links": {
    "self": "https://api.authorworks.local/v1/books?page=1&per_page=20",
    "next": "https://api.authorworks.local/v1/books?page=2&per_page=20",
    "prev": null
  }
}
```

### Collection Responses

Collections must always be represented as arrays within the `data` field:

```json
{
  "data": [
    { "id": "book1", "title": "First Book" },
    { "id": "book2", "title": "Second Book" }
  ],
  "meta": { ... },
  "pagination": { ... },
  "links": { ... }
}
```

### Field Naming Conventions

- Use camelCase for all field names
- Ensure consistent naming across services
- Use descriptive but concise field names
- Prefer full words over abbreviations
- Use ISO formats for dates and times: `YYYY-MM-DDThh:mm:ss.sssZ`

## Status Codes and Error Handling

Service implementations MUST use appropriate HTTP status codes:

| Status Code | Description | Usage |
|-------------|-------------|-------|
| 200 OK | Success | For successful GET, PUT, PATCH |
| 201 Created | Resource created | For successful POST that creates a resource |
| 204 No Content | Success with no response body | For successful DELETE |
| 400 Bad Request | Invalid request | Malformed request syntax |
| 401 Unauthorized | Authentication failure | Invalid or missing authentication |
| 403 Forbidden | Authorization failure | Valid authentication but insufficient permissions |
| 404 Not Found | Resource not found | Resource doesn't exist |
| 409 Conflict | Resource conflict | Request conflicts with resource state |
| 422 Unprocessable Entity | Validation error | Input validation failures |
| 429 Too Many Requests | Rate limit exceeded | Too many requests in a given time period |
| 500 Internal Server Error | Server error | Unexpected server-side error |
| 503 Service Unavailable | Service unavailable | Temporary outage or maintenance |

## Error Response Format

Error responses must follow the standard format as defined in the Error Handling Standard:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field1": "Error related to this field",
      "field2": "Error related to this field"
    },
    "request_id": "unique-request-identifier",
    "documentation_url": "https://docs.authorworks.com/errors/ERROR_CODE"
  }
}
```

### Error Codes

Error codes must follow the pattern: `SERVICE_CATEGORY_SPECIFIC`

Examples:
- `USER_AUTH_INVALID_CREDENTIALS`
- `CONTENT_VALIDATION_EMPTY_TITLE`
- `STORAGE_QUOTA_EXCEEDED`

## Authentication

### JWT Authentication

- All services must use JWT-based authentication
- Tokens should be obtained from the User Service
- Tokens must be validated against the User Service's public key
- Include appropriate CORS headers for browser clients

### Token Standards

- **Access tokens**: Short-lived (1 hour) with appropriate scope
- **Refresh tokens**: Longer-lived (30 days) for obtaining new access tokens
- **Token renewal**: Implemented using refresh tokens

## Rate Limiting

### Implementation

- All services must implement rate limiting
- Based on authenticated user or client IP
- Include standard headers in responses:

| Header | Description |
|--------|-------------|
| X-RateLimit-Limit | Maximum requests per time window |
| X-RateLimit-Remaining | Remaining requests in current window |
| X-RateLimit-Reset | Time when the current window resets (Unix timestamp) |

### Response Format

When rate limit is exceeded (429 status):

```json
{
  "error": {
    "code": "SERVICE_RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Please try again later.",
    "request_id": "unique-request-identifier",
    "details": {
      "retry_after": 30
    }
  }
}
```

## CORS Configuration

All APIs must support CORS with the following headers:

```
Access-Control-Allow-Origin: https://app.authorworks.com
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, X-Request-ID
Access-Control-Max-Age: 86400
```

## API Documentation

### OpenAPI Specification

- All services must provide OpenAPI 3.0+ specification
- Specifications should be available at `/v1/api-docs.json`
- Schema definitions should be comprehensive and include examples
- Authentication requirements must be clearly documented

### Documentation Portal

- API documentation will be centralized in the [authorworks-docs](https://github.com/authorworks/authorworks-docs) repository
- All services must keep their API documentation up-to-date
- Include code examples for common operations

## Implementation Checklist

When implementing a new API or endpoint, follow this checklist:

1. ☐ URL follows the standard structure
2. ☐ Uses appropriate HTTP methods
3. ☐ Implements standard request/response format
4. ☐ Returns correct status codes
5. ☐ Includes error handling with standard error format
6. ☐ Properly authenticates and authorizes requests
7. ☐ Implements rate limiting
8. ☐ Includes appropriate CORS headers
9. ☐ Documents API with OpenAPI specification
10. ☐ Adds implementation details to service documentation

## Migration Strategy

Services not yet conforming to these standards should implement changes in this order:

1. Standardize error response format
2. Align URL structure and versioning
3. Standardize response structure
4. Implement rate limiting
5. Provide OpenAPI documentation

## Appendix: API Endpoint Examples

### Resource Collection (GET /v1/books)

```
GET /v1/books?page=1&per_page=20 HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

Response:
```json
{
  "data": [
    {
      "id": "book-123",
      "title": "The Great Adventure",
      "authorId": "user-456",
      "publishedAt": "2023-05-10T15:30:00.000Z",
      "status": "published"
    },
    ...
  ],
  "meta": {
    "request_id": "req-789"
  },
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 42,
    "total_pages": 3
  },
  "links": {
    "self": "https://api.authorworks.local/v1/books?page=1&per_page=20",
    "next": "https://api.authorworks.local/v1/books?page=2&per_page=20",
    "prev": null
  }
}
```

### Resource Detail (GET /v1/books/{id})

```
GET /v1/books/book-123 HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

Response:
```json
{
  "data": {
    "id": "book-123",
    "title": "The Great Adventure",
    "authorId": "user-456",
    "publishedAt": "2023-05-10T15:30:00.000Z",
    "status": "published",
    "content": "Once upon a time...",
    "metadata": {
      "isbn": "978-3-16-148410-0",
      "genre": "fantasy",
      "pageCount": 320
    }
  },
  "meta": {
    "request_id": "req-789"
  }
}
```

### Resource Creation (POST /v1/books)

```
POST /v1/books HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "title": "New Book Title",
  "content": "Book content here...",
  "metadata": {
    "genre": "mystery",
    "tags": ["thriller", "suspense"]
  }
}
```

Response:
```json
{
  "data": {
    "id": "book-456",
    "title": "New Book Title",
    "authorId": "user-789",
    "createdAt": "2023-06-15T12:34:56.000Z",
    "status": "draft",
    "content": "Book content here...",
    "metadata": {
      "genre": "mystery",
      "tags": ["thriller", "suspense"]
    }
  },
  "meta": {
    "request_id": "req-abc"
  }
}
```

### Resource Update (PATCH /v1/books/{id})

```
PATCH /v1/books/book-456 HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "title": "Updated Book Title",
  "metadata": {
    "tags": ["thriller", "suspense", "mystery"]
  }
}
```

Response:
```json
{
  "data": {
    "id": "book-456",
    "title": "Updated Book Title",
    "authorId": "user-789",
    "createdAt": "2023-06-15T12:34:56.000Z",
    "updatedAt": "2023-06-15T13:45:12.000Z",
    "status": "draft",
    "content": "Book content here...",
    "metadata": {
      "genre": "mystery",
      "tags": ["thriller", "suspense", "mystery"]
    }
  },
  "meta": {
    "request_id": "req-def"
  }
}
```

### Resource Deletion (DELETE /v1/books/{id})

```
DELETE /v1/books/book-456 HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

Response:
```
HTTP/1.1 204 No Content
```

### Resource Operation (POST /v1/books/{id}/publish)

```
POST /v1/books/book-123/publish HTTP/1.1
Host: api.authorworks.local
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "publishDate": "2023-06-20T00:00:00.000Z",
  "channels": ["kindle", "print", "web"]
}
```

Response:
```json
{
  "data": {
    "id": "book-123",
    "title": "The Great Adventure",
    "authorId": "user-456",
    "createdAt": "2023-05-01T10:20:30.000Z",
    "publishedAt": "2023-06-20T00:00:00.000Z",
    "status": "publishing",
    "channels": ["kindle", "print", "web"],
    "estimatedCompletionTime": "2023-06-20T01:30:00.000Z"
  },
  "meta": {
    "request_id": "req-ghi"
  }
}
```

## Version History

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0.0   | 2023-06-15 | Architecture Team | Initial version of API standards |
| 1.1.0   | 2023-07-01 | Architecture Team | Added comprehensive examples and expanded error handling section | 