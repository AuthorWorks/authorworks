use axum::{
    routing::{get, post},
    Router,
    Json,
    http::{StatusCode, header},
    response::IntoResponse,
    extract::State,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, path::PathBuf};
use tower_http::{
    cors::{CorsLayer, Any},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    output_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

#[derive(Serialize, Deserialize)]
struct GenerateBookRequest {
    title: String,
    #[serde(default)]
    auto_generate: bool,
}

#[derive(Serialize, Deserialize)]
struct GenerateBookResponse {
    message: String,
    book_id: String,
    status: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,authorworks_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let output_dir = PathBuf::from("/app/output");
    std::fs::create_dir_all(&output_dir)?;

    let state = AppState { output_dir };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health and status
        .route("/health", get(health))
        .route("/", get(root))
        .route("/api/status", get(api_status))
        
        // Book generation API
        .route("/api/books/generate", post(generate_book))
        .route("/api/books", get(list_books))
        
        // Serve static UI
        .nest_service("/app", ServeDir::new("/app/public"))
        .fallback_service(ServeDir::new("/app/public"))
        
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state));

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🚀 AuthorWorks API server listening on http://{}:{}", host, port);
    tracing::info!("📚 Book output directory: {:?}", "/app/output");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "AuthorWorks Platform",
        "version": "1.0.0",
        "description": "AI-Powered Book Generation Platform",
        "endpoints": {
            "health": "/health",
            "api_status": "/api/status",
            "generate_book": "POST /api/books/generate",
            "list_books": "GET /api/books"
        }
    }))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "authorworks-platform".to_string(),
        version: "1.0.0".to_string(),
    })
}

async fn api_status() -> Json<serde_json::Value> {
    let anthropic_configured = std::env::var("ANTHROPIC_API_KEY").is_ok();
    
    Json(serde_json::json!({
        "status": "operational",
        "features": {
            "book_generation": anthropic_configured,
            "storage": true,
            "api": true
        },
        "configuration": {
            "anthropic_api": if anthropic_configured { "configured" } else { "not_configured" },
            "database": std::env::var("DATABASE_URL").is_ok(),
            "redis": std::env::var("REDIS_URL").is_ok()
        }
    }))
}

async fn generate_book(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateBookRequest>,
) -> Result<Json<GenerateBookResponse>, (StatusCode, String)> {
    tracing::info!("📖 Generating book: {}", payload.title);
    
    // Note: Book generation is async and long-running
    // In production, this would be a background job
    // For now, return immediate response indicating job started
    
    let book_id = format!("book_{}", chrono::Utc::now().timestamp());
    
    Ok(Json(GenerateBookResponse {
        message: format!("Book generation started for: {}", payload.title),
        book_id,
        status: "pending".to_string(),
    }))
}

async fn list_books(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let books = std::fs::read_dir(&state.output_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    
    Ok(Json(serde_json::json!({
        "books": books,
        "total": books.len()
    })))
}
