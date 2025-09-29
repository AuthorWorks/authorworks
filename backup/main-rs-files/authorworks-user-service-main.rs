use axum::{
    http::HeaderValue,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "authorworks_user_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build our application with routes
    let mut app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/users", get(list_users));

    // CORS configuration
    let cors = match std::env::var("ALLOWED_ORIGINS") {
        Ok(val) if !val.trim().is_empty() => {
            let origins: Vec<HeaderValue> = val
                .split(',')
                .filter_map(|o| HeaderValue::from_str(o.trim()).ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(AllowMethods::any())
                .allow_headers(AllowHeaders::any())
        }
        _ => CorsLayer::permissive(),
    };
    app = app.layer(cors);

    // Run the app
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("SERVICE_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3001);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("invalid HOST/PORT");
    tracing::info!("AuthorWorks User Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Basic handler that responds with service info
async fn root() -> Json<Value> {
    Json(json!({
        "service": "AuthorWorks User Service",
        "version": "0.1.0",
        "status": "running"
    }))
}

// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// List users endpoint (placeholder)
async fn list_users() -> Json<Value> {
    Json(json!({
        "users": [],
        "message": "User service is ready for implementation"
    }))
} 
