//! HTTP server wrapper for the book generator engine
//!
//! This provides a REST API for generating books using the core engine.

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use axum::body::Body;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use book_generator::{Config, generate_book_with_dir, render_book, generate_pdf_and_epub};
use book_generator::utils::logging::TokenTracker;

// Job status tracking
#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub id: String,
    pub book_id: Option<String>,
    pub status: String,
    pub phase: String,
    pub current_step: String,
    pub progress: f32,
    pub error: Option<String>,
    pub output_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub book_id: String,
    pub title: String,
    pub braindump: Option<String>,
    pub genre: Option<String>,
    pub style: Option<String>,
    pub characters: Option<String>,
    pub synopsis: Option<String>,
    pub chapter_count: Option<usize>,
    pub author_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// Shared state for tracking jobs
type JobStore = Arc<RwLock<HashMap<String, JobStatus>>>;

struct AppState {
    jobs: JobStore,
    output_base: PathBuf,
    database_url: Option<String>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "book_generator_server=info,tower_http=info".into()),
        )
        .init();

    // Load environment
    dotenvy::dotenv().ok();

    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let output_base = env::var("OUTPUT_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/data/books"));
    let database_url = env::var("DATABASE_URL").ok();

    // Create output directory
    std::fs::create_dir_all(&output_base).expect("Failed to create output directory");

    let state = Arc::new(AppState {
        jobs: Arc::new(RwLock::new(HashMap::new())),
        output_base,
        database_url,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_prometheus))
        .route("/api/generate", post(start_generation))
        .route("/api/jobs/:job_id", get(get_job_status))
        .route("/api/jobs/:job_id/cancel", post(cancel_job))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    tracing::info!("Book generator server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Prometheus exposition format for homelab monitoring.
async fn metrics_prometheus() -> Response {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let body = format!(
        "# HELP authorworks_book_generator_up Service is running (1 = up).\n\
         # TYPE authorworks_book_generator_up gauge\n\
         authorworks_book_generator_up 1 {}\n",
        timestamp
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8; version=0.0.4")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .unwrap()
}

async fn start_generation(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, (StatusCode, String)> {
    let job_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Create initial job status
    let job_status = JobStatus {
        id: job_id.clone(),
        book_id: Some(request.book_id.clone()),
        status: "pending".to_string(),
        phase: "initializing".to_string(),
        current_step: "Starting book generation".to_string(),
        progress: 0.0,
        error: None,
        output_path: None,
        created_at: now.clone(),
        updated_at: now,
    };

    // Store job
    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_status);
    }

    // Spawn generation task
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_generation(state_clone, job_id_clone, request).await;
    });

    Ok(Json(GenerateResponse {
        job_id,
        status: "started".to_string(),
        message: "Book generation started. Poll job status for updates.".to_string(),
    }))
}

async fn run_generation(state: Arc<AppState>, job_id: String, request: GenerateRequest) {
    // Helper to update job status
    let update_status = |jobs: &mut HashMap<String, JobStatus>, phase: &str, step: &str, progress: f32| {
        if let Some(job) = jobs.get_mut(&job_id) {
            job.phase = phase.to_string();
            job.current_step = step.to_string();
            job.progress = progress;
            job.status = "running".to_string();
            job.updated_at = chrono::Utc::now().to_rfc3339();
        }
    };

    // Update to running
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "setup", "Initializing configuration", 0.05);
    }

    // Create output directory for this book
    let output_dir = state.output_base.join(&request.book_id);
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = "failed".to_string();
            job.error = Some(format!("Failed to create output directory: {}", e));
            job.updated_at = chrono::Utc::now().to_rfc3339();
        }
        return;
    }

    // Write initial metadata with user inputs
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "braindump", "Processing creative ideas", 0.10);
    }

    // Create metadata file with user-provided context
    let metadata_content = format!(
        r#"# Book Metadata

## Title
{}

## Braindump
{}

## Genre
{}

## Style
{}

## Characters
{}

## Synopsis
{}
"#,
        request.title,
        request.braindump.as_deref().unwrap_or(""),
        request.genre.as_deref().unwrap_or(""),
        request.style.as_deref().unwrap_or(""),
        request.characters.as_deref().unwrap_or(""),
        request.synopsis.as_deref().unwrap_or("")
    );

    if let Err(e) = std::fs::write(output_dir.join("metadata.md"), metadata_content) {
        tracing::error!("Failed to write metadata: {}", e);
    }

    // Create config with appropriate settings
    let mut config = Config::default();
    config.auto_generate = true;
    if let Some(count) = request.chapter_count {
        config.max_chapters = count;
    }

    // Update progress: Genre analysis
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "genre", "Analyzing genre conventions", 0.15);
    }

    // Update progress: Style
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "style", "Defining writing style", 0.20);
    }

    // Update progress: Characters
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "characters", "Developing character profiles", 0.25);
    }

    // Update progress: Synopsis
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "synopsis", "Crafting story synopsis", 0.30);
    }

    // Run the actual book generation
    {
        let mut jobs = state.jobs.write().await;
        update_status(&mut jobs, "outline", "Generating book outline", 0.35);
    }

    let generation_result = generate_book_with_dir(
        request.title.clone(),
        &config,
        &output_dir,
        true, // auto_generate
    ).await;

    match generation_result {
        Ok((book, token_tracker)) => {
            // Update progress: Chapters
            {
                let mut jobs = state.jobs.write().await;
                update_status(&mut jobs, "chapters", "Structuring chapter outlines", 0.50);
            }

            // Update progress: Scenes
            {
                let mut jobs = state.jobs.write().await;
                update_status(&mut jobs, "scenes", "Planning scene breakdowns", 0.60);
            }

            // Update progress: Content
            {
                let mut jobs = state.jobs.write().await;
                update_status(&mut jobs, "content", "Generating chapter content", 0.70);
            }

            // Render the book
            {
                let mut jobs = state.jobs.write().await;
                update_status(&mut jobs, "rendering", "Rendering book to HTML", 0.80);
            }

            if let Err(e) = render_book(&book, &output_dir, Some(&token_tracker)).await {
                tracing::error!("Failed to render book: {}", e);
            }

            // Generate PDF and EPUB
            {
                let mut jobs = state.jobs.write().await;
                update_status(&mut jobs, "export", "Generating PDF and EPUB", 0.90);
            }

            let author = request.author_name.as_deref().unwrap_or("AuthorWorks User");
            if let Err(e) = generate_pdf_and_epub(&output_dir, &request.title, author) {
                tracing::error!("Failed to generate PDF/EPUB: {}", e);
            }

            // Mark as complete
            {
                let mut jobs = state.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = "completed".to_string();
                    job.phase = "complete".to_string();
                    job.current_step = "Book generation complete".to_string();
                    job.progress = 1.0;
                    job.output_path = Some(output_dir.to_string_lossy().to_string());
                    job.updated_at = chrono::Utc::now().to_rfc3339();
                }
            }

            tracing::info!("Book generation completed for job {}", job_id);
        }
        Err(e) => {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = "failed".to_string();
                job.error = Some(format!("Generation failed: {}", e));
                job.updated_at = chrono::Utc::now().to_rfc3339();
            }
            tracing::error!("Book generation failed for job {}: {}", job_id, e);
        }
    }
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatus>, (StatusCode, String)> {
    let jobs = state.jobs.read().await;
    jobs.get(&job_id)
        .cloned()
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Job not found".to_string()))
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatus>, (StatusCode, String)> {
    let mut jobs = state.jobs.write().await;
    if let Some(job) = jobs.get_mut(&job_id) {
        if job.status == "running" || job.status == "pending" {
            job.status = "cancelled".to_string();
            job.updated_at = chrono::Utc::now().to_rfc3339();
        }
        Ok(Json(job.clone()))
    } else {
        Err((StatusCode::NOT_FOUND, "Job not found".to_string()))
    }
}
