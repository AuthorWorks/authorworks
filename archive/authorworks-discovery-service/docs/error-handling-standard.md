# AuthorWorks Error Handling Standard

## Overview

This document outlines the standardized approach to error handling across all AuthorWorks microservices. Consistent error handling is crucial for debugging, monitoring, and providing clear feedback to developers and end-users.

## Error Response Structure

All API errors should follow this JSON structure:

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

### Fields

| Field | Description | Required |
|-------|-------------|----------|
| `code` | Unique error code identifier | Yes |
| `message` | Human-readable error description | Yes |
| `details` | Field-specific validation errors or additional information | No |
| `request_id` | Unique identifier for the request (for logging/tracking) | Yes |
| `documentation_url` | Link to error documentation | No |

## HTTP Status Codes

Services should use standard HTTP status codes consistently:

| Status Code | Description | Usage |
|-------------|-------------|-------|
| 400 | Bad Request | Invalid request parameters or format |
| 401 | Unauthorized | Missing or invalid authentication |
| 403 | Forbidden | Authentication succeeded but insufficient permissions |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict (e.g., already exists) |
| 422 | Unprocessable Entity | Validation errors |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Unexpected server errors |
| 503 | Service Unavailable | Service temporarily unavailable (maintenance, overloaded) |

## Error Codes

Error codes should follow the format: `SERVICE_CATEGORY_SPECIFIC`

Examples:
- `USER_AUTH_INVALID_CREDENTIALS`
- `CONTENT_VALIDATION_EMPTY_TITLE`
- `STORAGE_QUOTA_EXCEEDED`

## Circuit Breaker Pattern

All services should implement the circuit breaker pattern as observed in the API Gateway implementation:

```rust
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_threshold: usize,
    reset_timeout: Duration,
}

pub enum CircuitState {
    Closed { failures: 0 },
    Open { opened_at: Instant },
    HalfOpen,
}

// Circuit breaker methods:
// - record_success()
// - record_failure()
// - is_closed()
```

Circuit breaker state transitions:
1. **Closed**: Normal operation, counting failures
2. **Open**: When failures exceed threshold, reject requests immediately
3. **Half-Open**: After timeout, allow one test request
4. **Closed/Open**: Based on test request success/failure

## Logging Standards

Error logs should include:

1. Error code
2. Error message
3. Request ID
4. Stack trace (in development environments)
5. Relevant context (user ID, resource ID, etc.)

Example:
```
ERROR [2023-06-15T12:34:56Z] [request_id=abcd1234] [user_id=user123] USER_AUTH_INVALID_CREDENTIALS: Invalid username or password provided
```

## Validation Error Handling

For validation errors:

1. Use HTTP 422 Unprocessable Entity
2. Include detailed field-specific errors in the `details` object
3. Use consistent field names matching the request parameters

Example:
```json
{
  "error": {
    "code": "CONTENT_VALIDATION_ERROR",
    "message": "Validation failed",
    "details": {
      "title": "Title cannot be empty",
      "word_count": "Word count must be at least 100"
    },
    "request_id": "abcd1234"
  }
}
```

## Global Error Handler

Each service should implement a global error handler to:

1. Catch uncaught exceptions
2. Format errors according to the standard
3. Log errors with appropriate context
4. Hide internal implementation details in production

## Integration Between Services

When service A calls service B:

1. Service A should handle error responses from service B
2. Circuit breaker pattern should be used to prevent cascading failures
3. Retry logic should be implemented where appropriate
4. Request IDs should be propagated across service boundaries

## Event Processing Errors

For errors during event processing:

1. Implement dead letter queues
2. Provide clear error information in the failed event
3. Implement retry mechanisms with exponential backoff
4. Alert on persistent failures

## Client-Side Error Handling

The UI shell should:

1. Process error responses consistently
2. Display user-friendly error messages
3. Provide appropriate recovery actions
4. Log client-side errors for analysis

## Rate Limiting Errors

Rate limit error responses should include:

1. Standard error format with code `SERVICE_RATE_LIMIT_EXCEEDED`
2. `Retry-After` header indicating when to retry
3. Details about current limits and usage

## Implementation Guidelines

When implementing error handling:

1. Use structured error types within services
2. Convert internal errors to standard format at API boundaries
3. Never expose internal stack traces to clients in production
4. Generate unique request IDs and propagate through the request chain
5. Include comprehensive error handling in API documentation

## Testing Error Scenarios

Service testing should include:

1. Validation of error response structure
2. Tests for all defined error codes
3. Circuit breaker behavior verification
4. Rate limiting behavior tests
5. Integration tests for cross-service error handling

## Monitoring and Alerting

Error monitoring should:

1. Track error rates by code and service
2. Alert on unexpected error rate increases
3. Provide error dashboards for operations
4. Analyze error patterns

## Example Implementation

```rust
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
}

impl ErrorResponse {
    pub fn new(code: &str, message: &str, request_id: &str) -> Self {
        Self {
            error: ErrorDetails {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
                request_id: request_id.to_string(),
                documentation_url: Some(format!("https://docs.authorworks.com/errors/{}", code)),
            },
        }
    }
    
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }
}
``` 