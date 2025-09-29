use axum::{
    http::HeaderValue,
    routing::{get, post},
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
                .unwrap_or_else(|_| "authorworks_content_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build our application with routes
    let mut app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/content", get(list_content).post(create_content))
        .route("/api/v1/content/:id", get(get_content));

    let cors = match std::env::var("ALLOWED_ORIGINS") {
        Ok(val) if !val.trim().is_empty() => {
            let origins: Vec<HeaderValue> = val.split(',').filter_map(|o| HeaderValue::from_str(o.trim()).ok()).collect();
            CorsLayer::new().allow_origin(AllowOrigin::list(origins)).allow_methods(AllowMethods::any()).allow_headers(AllowHeaders::any())
        }
        _ => CorsLayer::permissive(),
    };
    app = app.layer(cors);

    // Run the server
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("SERVICE_PORT").or_else(|_| std::env::var("PORT")).ok().and_then(|s| s.parse().ok()).unwrap_or(3002);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("invalid HOST/PORT");
    tracing::info!("AuthorWorks Content Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({
        "service": "AuthorWorks Content Service",
        "version": "0.1.0",
        "status": "running"
    }))
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn list_content() -> Json<Value> {
    Json(json!({
        "content": [],
        "total": 0,
        "message": "Content listing endpoint - ready for implementation"
    }))
}

async fn create_content(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({
        "message": "Content creation endpoint - ready for implementation",
        "received": payload
    }))
}

async fn get_content() -> Json<Value> {
    Json(json!({
        "message": "Content retrieval endpoint - ready for implementation"
    }))
} 
