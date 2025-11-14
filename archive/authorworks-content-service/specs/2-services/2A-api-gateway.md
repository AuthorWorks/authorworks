# Technical Specification: 2A - API Gateway Service

## Overview

This specification details the API Gateway service for the AuthorWorks platform. The gateway serves as the unified entry point for all client requests, handling routing, authentication, rate limiting, and monitoring.

## Objectives

- Create a unified entry point for all client requests
- Implement centralized authentication and authorization
- Provide request routing to appropriate backend services
- Add rate limiting and throttling capabilities
- Establish request/response logging and monitoring
- Implement circuit breaking for service resilience

## Requirements

### 1. Core Gateway Functionality

Implement an API Gateway service using Rust with Axum framework:

#### Request Routing

Create a routing system that directs requests to appropriate services:

```rust
// Example of route configuration in Axum
async fn configure_routes(app: Router, config: &Config) -> Router {
    app
        // User service routes
        .route("/v1/users/*path", 
            get(proxy_service)
                .post(proxy_service)
                .put(proxy_service)
                .patch(proxy_service)
                .delete(proxy_service))
        .route_layer(middleware::from_fn_with_state(
            config.clone(),
            |req: Request, next: axum::middleware::Next, config: Config| async move {
                route_to_service(req, next, config, "user-service").await
            }
        ))
        
        // Content service routes
        .route("/v1/books/*path", 
            get(proxy_service)
                .post(proxy_service)
                .put(proxy_service)
                .patch(proxy_service)
                .delete(proxy_service))
        .route_layer(middleware::from_fn_with_state(
            config.clone(),
            |req: Request, next: axum::middleware::Next, config: Config| async move {
                route_to_service(req, next, config, "content-service").await
            }
        ))
        
        // Additional service routes...
}

async fn route_to_service(
    mut req: Request,
    next: axum::middleware::Next,
    config: Config,
    service_name: &str,
) -> Result<Response, StatusCode> {
    // Get service URL from config or service discovery
    let service_url = config.get_service_url(service_name)
        .ok_or(StatusCode::BAD_GATEWAY)?;
    
    // Set X-Forwarded headers
    req.headers_mut().insert(
        HeaderName::from_static("x-forwarded-host"),
        req.headers()
            .get(header::HOST)
            .cloned()
            .unwrap_or_else(|| HeaderValue::from_static("unknown")),
    );
    
    // Store target service in request extensions
    req.extensions_mut().insert(TargetService {
        name: service_name.to_string(),
        url: service_url.clone(),
    });
    
    // Continue with request processing
    next.run(req).await
}

async fn proxy_service(
    State(client): State<Client>,
    req: Request,
) -> Result<Response, StatusCode> {
    let target = req.extensions()
        .get::<TargetService>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Extract path and query from original request
    let uri = req.uri();
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    
    // Create target URL
    let url = format!("{}{}{}", target.url, path, query);
    
    // Forward request to target service
    let mut proxy_req = hyper::Request::builder()
        .method(req.method().clone())
        .uri(url);
    
    // Copy headers from original request
    for (name, value) in req.headers() {
        if !EXCLUDED_HEADERS.contains(&name.as_str()) {
            proxy_req = proxy_req.header(name, value);
        }
    }
    
    // Add proxy headers
    proxy_req = proxy_req.header("X-Forwarded-For", get_client_ip(&req));
    
    // Copy body from original request
    let proxy_req = proxy_req.body(req.into_body())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Send request to target service
    client.request(proxy_req)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)
}
```

#### Service Discovery

Implement service discovery for locating backend services:

1. **Static Configuration**:
   - Environment-based service URL mapping
   - Configurable through environment variables or config files

2. **Dynamic Service Discovery**:
   - Kubernetes service discovery integration
   - Service health checks and availability monitoring

```rust
pub struct ServiceRegistry {
    services: RwLock<HashMap<String, ServiceInfo>>,
    discovery_client: Arc<dyn ServiceDiscovery>,
    refresh_interval: Duration,
}

impl ServiceRegistry {
    pub fn new(discovery_client: Arc<dyn ServiceDiscovery>, refresh_interval: Duration) -> Self {
        let registry = Self {
            services: RwLock::new(HashMap::new()),
            discovery_client,
            refresh_interval,
        };
        
        // Start background refresh task
        let registry_clone = registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            loop {
                interval.tick().await;
                if let Err(e) = registry_clone.refresh_services().await {
                    tracing::error!("Failed to refresh services: {}", e);
                }
            }
        });
        
        registry
    }
    
    pub async fn refresh_services(&self) -> Result<(), Error> {
        let services = self.discovery_client.discover_services().await?;
        let mut registry = self.services.write().await;
        *registry = services;
        Ok(())
    }
    
    pub async fn get_service(&self, name: &str) -> Option<ServiceInfo> {
        let registry = self.services.read().await;
        registry.get(name).cloned()
    }
}

#[async_trait]
pub trait ServiceDiscovery: Send + Sync {
    async fn discover_services(&self) -> Result<HashMap<String, ServiceInfo>, Error>;
}

// Kubernetes implementation
pub struct KubernetesServiceDiscovery {
    client: kube::Client,
    namespace: String,
}

#[async_trait]
impl ServiceDiscovery for KubernetesServiceDiscovery {
    async fn discover_services(&self) -> Result<HashMap<String, ServiceInfo>, Error> {
        let api = kube::Api::<k8s_openapi::api::core::v1::Service>::namespaced(
            self.client.clone(),
            &self.namespace,
        );
        
        let services = api.list(&kube::api::ListParams::default()).await?;
        let mut result = HashMap::new();
        
        for service in services.items {
            if let Some(metadata) = service.metadata {
                if let Some(name) = metadata.name {
                    if let Some(spec) = service.spec {
                        if let Some(cluster_ip) = spec.cluster_ip {
                            let port = spec.ports.and_then(|ports| {
                                ports.first().map(|port| port.port)
                            }).unwrap_or(80);
                            
                            let url = format!("http://{}:{}", cluster_ip, port);
                            
                            result.insert(name.clone(), ServiceInfo {
                                name,
                                url,
                                healthy: true,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(result)
    }
}
```

### 2. Authentication and Authorization

Implement authentication middleware for all routes:

#### JWT Authentication

Create middleware for validating JSON Web Tokens:

```rust
pub async fn jwt_auth<B>(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Extract token from header
    let token = auth.token();
    
    // Validate JWT
    let claims = match validate_token(token, &state.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Check token expiration
    let now = chrono::Utc::now().timestamp() as usize;
    if claims.exp < now {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Extract user information
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Create authenticated user
    let user = AuthUser {
        id: user_id,
        roles: claims.roles,
        exp: claims.exp,
    };
    
    // Add user to request extensions
    req.extensions_mut().insert(user);
    
    // Continue with request processing
    Ok(next.run(req).await)
}

fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, JwtError> {
    let validation = Validation::new(Algorithm::HS256);
    let claims = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(claims.claims)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub roles: Vec<String>,
    pub exp: usize,
}
```

#### Role-Based Access Control

Implement role-based authorization for protected endpoints:

```rust
pub async fn require_role<B>(
    req: Request<B>,
    next: Next<B>,
    required_roles: &[&str],
) -> Result<Response, StatusCode> {
    // Get authenticated user from request extensions
    let user = req.extensions()
        .get::<AuthUser>()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Check if user has any of the required roles
    let has_required_role = required_roles.iter()
        .any(|role| user.roles.contains(&role.to_string()));
    
    if !has_required_role {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Continue with request processing
    Ok(next.run(req).await)
}

// Example usage in route definition
app.route("/v1/admin/*path", 
    get(proxy_service)
        .post(proxy_service)
        .put(proxy_service)
        .patch(proxy_service)
        .delete(proxy_service))
    .route_layer(middleware::from_fn(|req, next| require_role(req, next, &["admin"])))
```

### 3. Rate Limiting and Throttling

Implement rate limiting to protect backend services:

#### IP-Based Rate Limiting

```rust
pub struct RateLimiter {
    store: Arc<RwLock<HashMap<String, RateLimitState>>>,
    config: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitState {
    tokens: usize,
    last_refill: Instant,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    rate: usize,         // Tokens per second
    burst: usize,        // Maximum burst size
    per_route: bool,     // Apply per route or globally
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let mut store = self.store.write().await;
        let now = Instant::now();
        
        let state = store.entry(key.to_string())
            .or_insert_with(|| RateLimitState {
                tokens: self.config.burst,
                last_refill: now,
            });
        
        // Calculate tokens to add based on time passed
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let tokens_to_add = (elapsed * self.config.rate as f64) as usize;
        
        if tokens_to_add > 0 {
            state.tokens = (state.tokens + tokens_to_add).min(self.config.burst);
            state.last_refill = now;
        }
        
        if state.tokens > 0 {
            state.tokens -= 1;
            true
        } else {
            false
        }
    }
}

pub async fn rate_limit_middleware<B>(
    State(limiter): State<RateLimiter>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Get client IP address
    let ip = get_client_ip(&req);
    
    // Create rate limit key (IP or IP + route)
    let key = if limiter.config.per_route {
        format!("{}:{}", ip, req.uri().path())
    } else {
        ip
    };
    
    // Check rate limit
    if !limiter.check_rate_limit(&key).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Continue with request processing
    Ok(next.run(req).await)
}
```

#### User-Based Rate Limiting

```rust
pub async fn user_rate_limit<B>(
    State(limiter): State<RateLimiter>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Get authenticated user from request extensions
    let user = req.extensions()
        .get::<AuthUser>()
        .map(|user| user.id.to_string())
        .unwrap_or_else(|| get_client_ip(&req));
    
    // Create rate limit key
    let key = if limiter.config.per_route {
        format!("{}:{}", user, req.uri().path())
    } else {
        user
    };
    
    // Check rate limit
    if !limiter.check_rate_limit(&key).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Continue with request processing
    Ok(next.run(req).await)
}
```

### 4. Circuit Breaking

Implement circuit breaking for resilient service calls:

```rust
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_threshold: usize,
    reset_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed {
        failures: usize,
    },
    Open {
        opened_at: Instant,
    },
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed { failures: 0 }),
            failure_threshold,
            reset_timeout,
        }
    }
    
    pub async fn allow_request(&self) -> bool {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed { .. } => true,
            CircuitState::Open { opened_at } => {
                if opened_at.elapsed() >= self.reset_timeout {
                    // Transition to half-open state
                    *state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        
        if let CircuitState::HalfOpen = *state {
            // Reset on successful request in half-open state
            *state = CircuitState::Closed { failures: 0 };
        } else if let CircuitState::Closed { failures } = *state {
            if failures > 0 {
                // Reset failure count on success
                *state = CircuitState::Closed { failures: 0 };
            }
        }
    }
    
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed { failures } => {
                if failures + 1 >= self.failure_threshold {
                    // Trip the circuit breaker
                    *state = CircuitState::Open { opened_at: Instant::now() };
                } else {
                    // Increment failure count
                    *state = CircuitState::Closed { failures: failures + 1 };
                }
            }
            CircuitState::HalfOpen => {
                // Trip the circuit again on failure in half-open state
                *state = CircuitState::Open { opened_at: Instant::now() };
            }
            CircuitState::Open { .. } => {
                // Already open, do nothing
            }
        }
    }
}

pub async fn circuit_breaker_middleware<B>(
    State(breakers): State<HashMap<String, Arc<CircuitBreaker>>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Get target service from request extensions
    let service = req.extensions()
        .get::<TargetService>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .name.clone();
    
    // Get circuit breaker for service
    let breaker = breakers.get(&service)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Check if request is allowed
    if !breaker.allow_request().await {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    
    // Process request
    let response = next.run(req).await;
    
    // Record success or failure
    if response.status().is_server_error() {
        breaker.record_failure().await;
    } else {
        breaker.record_success().await;
    }
    
    Ok(response)
}
```

### 5. Logging and Monitoring

Implement comprehensive logging and monitoring:

#### Request Logging

```rust
pub async fn request_logger<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    // Extract request ID or generate one
    let request_id = req.headers()
        .get("X-Request-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| {
            let id = Uuid::new_v4().to_string();
            // TODO: Add request ID to request extensions
            id
        });
    
    // Log request
    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Request started"
    );
    
    // Process request
    let response = next.run(req).await;
    
    // Log response
    let status = response.status().as_u16();
    let duration = start.elapsed();
    
    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );
    
    Ok(response)
}
```

#### Metrics Collection

```rust
pub async fn metrics_middleware<B>(
    State(metrics): State<Metrics>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    // Get target service from request extensions
    let service = req.extensions()
        .get::<TargetService>()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    
    // Process request
    let response = next.run(req).await;
    
    // Record metrics
    let status = response.status().as_u16();
    let duration = start.elapsed();
    
    // Increment request counter
    metrics.request_count
        .with_label_values(&[
            method.as_str(),
            uri.path(),
            &service,
            &status.to_string(),
        ])
        .inc();
    
    // Record request duration
    metrics.request_duration
        .with_label_values(&[
            method.as_str(),
            uri.path(),
            &service,
            &status.to_string(),
        ])
        .observe(duration.as_secs_f64());
    
    Ok(response)
}

pub struct Metrics {
    pub request_count: IntCounterVec,
    pub request_duration: HistogramVec,
    pub active_connections: IntGauge,
}

impl Metrics {
    pub fn new() -> Self {
        let request_count = IntCounterVec::new(
            Opts::new("gateway_requests_total", "Total number of requests processed"),
            &["method", "path", "service", "status"],
        ).expect("Failed to create request count metric");
        
        let request_duration = HistogramVec::new(
            HistogramOpts::new("gateway_request_duration_seconds", "Request duration in seconds"),
            &["method", "path", "service", "status"],
        ).expect("Failed to create request duration metric");
        
        let active_connections = IntGauge::new(
            "gateway_active_connections", "Number of active connections",
        ).expect("Failed to create active connections metric");
        
        Self {
            request_count,
            request_duration,
            active_connections,
        }
    }
    
    pub fn register(&self) -> Result<(), prometheus::Error> {
        prometheus::default_registry().register(Box::new(self.request_count.clone()))?;
        prometheus::default_registry().register(Box::new(self.request_duration.clone()))?;
        prometheus::default_registry().register(Box::new(self.active_connections.clone()))?;
        Ok(())
    }
}
```

### 6. API Gateway Configuration

Implement a flexible configuration system:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub services: HashMap<String, ServiceConfig>,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub url: String,
    pub timeout: Duration,
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry: Duration,
    pub issuer: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load configuration from environment variables
        let mut config = config::Config::new();
        
        // Set default values
        config.set_default("server.host", "0.0.0.0")?;
        config.set_default("server.port", 8080)?;
        
        // Add environment source with prefix
        config.merge(config::Environment::with_prefix("GATEWAY").separator("__"))?;
        
        // Try to load configuration file if specified
        if let Ok(path) = std::env::var("GATEWAY_CONFIG_FILE") {
            config.merge(config::File::with_name(&path))?;
        }
        
        // Convert to our config struct
        config.try_into()
    }
    
    pub fn get_service_url(&self, service: &str) -> Option<String> {
        self.services.get(service).map(|s| s.url.clone())
    }
}
```

## Implementation Steps

1. Create API Gateway project structure
2. Implement routing and service discovery
3. Add authentication and authorization
4. Implement rate limiting and circuit breaking
5. Add logging and metrics collection
6. Create configuration system
7. Implement comprehensive testing
8. Deploy and configure for different environments

## Technical Decisions

### Why Axum over other frameworks?

Axum was chosen over other Rust web frameworks for the API Gateway because:
- Performance optimization for async/await
- Tower-based middleware system for composability
- Efficient route matching for high-throughput applications
- First-class support for TypeScript frontend integration
- Hyper-based for HTTP/1.1 and HTTP/2 support

### Why a custom gateway over off-the-shelf solutions?

A custom gateway was selected over existing API gateway solutions because:
- Tight integration with our Rust-based microservices
- Full control over authentication and authorization flows
- Custom rate limiting and circuit breaking behavior
- Specialized logging and metrics for our observability stack
- Reduced overhead compared to proxy-based solutions

## Success Criteria

The API Gateway will be considered successfully implemented when:

1. All client requests are properly routed to backend services
2. Authentication and authorization are consistently applied
3. Rate limiting and circuit breaking protect backend services
4. Comprehensive logging and monitoring are available
5. Gateway performance meets or exceeds 1000 requests per second
6. All error scenarios are properly handled and reported 