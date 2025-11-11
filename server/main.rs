use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::Path as AxumPath,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health endpoints
        .route("/health", get(health))
        .route("/", get(root))
        
        // User service endpoints
        .route("/api/user/*path", get(user_handler).post(user_handler))
        .route("/api/users", get(list_users).post(create_user))
        
        // Content service endpoints  
        .route("/api/content/*path", get(content_handler).post(content_handler))
        
        // Storage service endpoints
        .route("/api/storage/*path", get(storage_handler).post(storage_handler))
        
        // Editor service endpoints
        .route("/api/editor/*path", get(editor_handler).post(editor_handler))
        
        // Messaging service endpoints
        .route("/api/messaging/*path", get(messaging_handler))
        
        // Discovery service endpoints
        .route("/api/discovery/*path", get(discovery_handler))
        
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("AuthorWorks server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "AuthorWorks AI-Assisted Content Creation Platform".to_string(),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "authorworks-platform".to_string(),
        version: "1.0.0".to_string(),
    })
}

async fn user_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("User service - path: {:?}", path),
    })
}

async fn list_users() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "users": [],
        "total": 0
    }))
}

async fn create_user() -> (StatusCode, Json<ApiResponse>) {
    (StatusCode::CREATED, Json(ApiResponse {
        message: "User created successfully".to_string(),
    }))
}

async fn content_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("Content service - path: {:?}", path),
    })
}

async fn storage_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("Storage service - path: {:?}", path),
    })
}

async fn editor_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("Editor service - path: {:?}", path),
    })
}

async fn messaging_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("Messaging service - path: {:?}", path),
    })
}

async fn discovery_handler(path: Option<AxumPath<String>>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: format!("Discovery service - path: {:?}", path),
    })
}
