//! HTTP server wrapper for the book generator engine
//!
//! This provides a REST API for generating books using the core engine.
//! Job state is persisted to disk (under OUTPUT_BASE/.jobs) so restarts do not
//! lose job history, and finished books expose their rendered artifacts for
//! download via /api/books/:book_id/files.

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Json, Response},
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

// Job status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Synopsis of the generated book, populated on completion.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub synopsis: Option<String>,
    /// Full chapter content, populated on completion so the caller can sync
    /// the book into its own storage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub chapters: Option<Vec<JobChapter>>,
    /// Downloadable artifacts (relative file names) in the book output dir.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobChapter {
    pub number: usize,
    pub title: String,
    pub content: String,
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
    /// Bring-your-own inference: OpenAI-compatible API base URL. When set,
    /// generation uses this endpoint instead of the platform default.
    pub llm_api_base: Option<String>,
    /// Model name to use with the override endpoint (or platform gateway).
    pub llm_model: Option<String>,
    /// API key for the override endpoint. Never falls back to the platform
    /// key when an override base URL is provided.
    pub llm_api_key: Option<String>,
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
}

impl AppState {
    fn jobs_dir(&self) -> PathBuf {
        self.output_base.join(".jobs")
    }
}

/// Applies per-request LLM overrides to the environment-derived config.
/// An override base URL switches the provider to the OpenAI-compatible path
/// and never reuses the platform API key.
fn apply_llm_overrides(config: &mut Config, request: &GenerateRequest) {
    if let Some(base) = request
        .llm_api_base
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.llm_provider = "openai".to_string();
        config.openai_api_base = base.trim_end_matches('/').to_string();
        config.openai_api_key = request
            .llm_api_key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("no-key")
            .to_string();
    }
    if let Some(model) = request
        .llm_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.model = model.to_string();
    }
}

/// Rejects path components that could escape the output directory.
fn is_safe_path_component(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

fn content_type_for(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "pdf" => "application/pdf",
        "epub" => "application/epub+zip",
        "html" => "text/html; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Lists downloadable artifacts (rendered outputs) in a book directory.
fn list_book_files(book_dir: &FsPath) -> Vec<String> {
    const DOWNLOADABLE: &[&str] = &["pdf", "epub", "html", "md", "json", "txt"];
    let mut files: Vec<String> = std::fs::read_dir(book_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| is_safe_path_component(name))
                .filter(|name| {
                    name.rsplit('.')
                        .next()
                        .map(|ext| DOWNLOADABLE.contains(&ext))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn persist_job(jobs_dir: &FsPath, job: &JobStatus) {
    if let Err(e) = std::fs::create_dir_all(jobs_dir) {
        tracing::warn!("Failed to create jobs dir: {}", e);
        return;
    }
    let path = jobs_dir.join(format!("{}.json", job.id));
    match serde_json::to_vec(job) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&path, bytes) {
                tracing::warn!("Failed to persist job {}: {}", job.id, e);
            }
        }
        Err(e) => tracing::warn!("Failed to serialize job {}: {}", job.id, e),
    }
}

/// Restores persisted jobs on startup. Jobs that were in-flight when the
/// process stopped are marked failed so callers don't poll forever; the
/// generator itself resumes from its on-disk phase cache when re-triggered.
fn load_persisted_jobs(jobs_dir: &FsPath) -> HashMap<String, JobStatus> {
    let mut jobs = HashMap::new();
    let Ok(entries) = std::fs::read_dir(jobs_dir) else {
        return jobs;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(mut job) = serde_json::from_slice::<JobStatus>(&bytes) else {
            tracing::warn!("Skipping unreadable job file {:?}", path);
            continue;
        };
        if job.status == "running" || job.status == "pending" {
            job.status = "failed".to_string();
            job.error = Some("Interrupted by service restart; re-trigger generation to resume".to_string());
            job.updated_at = chrono::Utc::now().to_rfc3339();
            persist_job(jobs_dir, &job);
        }
        jobs.insert(job.id.clone(), job);
    }
    jobs
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

    // Create output directory
    std::fs::create_dir_all(&output_base).expect("Failed to create output directory");

    let jobs_dir = output_base.join(".jobs");
    let restored = load_persisted_jobs(&jobs_dir);
    if !restored.is_empty() {
        tracing::info!("Restored {} persisted jobs", restored.len());
    }

    let state = Arc::new(AppState {
        jobs: Arc::new(RwLock::new(restored)),
        output_base,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_prometheus))
        .route("/api/generate", post(start_generation))
        .route("/api/jobs/:job_id", get(get_job_status))
        .route("/api/jobs/:job_id/cancel", post(cancel_job))
        .route("/api/books/:book_id/files", get(list_files))
        .route("/api/books/:book_id/files/:filename", get(download_file))
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
    if !is_safe_path_component(&request.book_id) {
        return Err((StatusCode::BAD_REQUEST, "Invalid book_id".to_string()));
    }

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
        synopsis: None,
        chapters: None,
        files: None,
    };

    // Store job
    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_status.clone());
    }
    persist_job(&state.jobs_dir(), &job_status);

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

/// Mutates a job under the write lock and persists the result to disk.
async fn update_job<F>(state: &Arc<AppState>, job_id: &str, mutate: F)
where
    F: FnOnce(&mut JobStatus),
{
    let snapshot = {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            mutate(job);
            job.updated_at = chrono::Utc::now().to_rfc3339();
            Some(job.clone())
        } else {
            None
        }
    };
    if let Some(job) = snapshot {
        persist_job(&state.jobs_dir(), &job);
    }
}

async fn run_generation(state: Arc<AppState>, job_id: String, request: GenerateRequest) {
    let set_progress = |phase: &'static str, step: &'static str, progress: f32| {
        let state = state.clone();
        let job_id = job_id.clone();
        async move {
            update_job(&state, &job_id, |job| {
                job.status = "running".to_string();
                job.phase = phase.to_string();
                job.current_step = step.to_string();
                job.progress = progress;
            })
            .await;
        }
    };

    let fail_job = |error: String| {
        let state = state.clone();
        let job_id = job_id.clone();
        async move {
            update_job(&state, &job_id, |job| {
                job.status = "failed".to_string();
                job.error = Some(error);
            })
            .await;
        }
    };

    set_progress("setup", "Initializing configuration", 0.05).await;

    // Create output directory for this book
    let output_dir = state.output_base.join(&request.book_id);
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        fail_job(format!("Failed to create output directory: {}", e)).await;
        return;
    }

    set_progress("braindump", "Processing creative ideas", 0.10).await;

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

    // Create config from environment variables (reads LLM_PROVIDER, OPENAI_API_BASE, etc.)
    let mut config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            fail_job(format!("Configuration error: {}", e)).await;
            return;
        }
    };
    config.auto_generate = true;
    if let Some(count) = request.chapter_count {
        config.max_chapters = count;
    }
    apply_llm_overrides(&mut config, &request);
    tracing::info!(
        "Job {} using provider={} base={} model={}",
        job_id,
        config.llm_provider,
        config.openai_api_base,
        config.model
    );

    set_progress("outline", "Generating book outline", 0.25).await;

    let generation_result = generate_book_with_dir(
        request.title.clone(),
        &config,
        &output_dir,
        true, // auto_generate
    ).await;

    match generation_result {
        Ok((book, token_tracker)) => {
            set_progress("rendering", "Rendering book to HTML", 0.80).await;

            if let Err(e) = render_book(&book, &output_dir, Some(&token_tracker)).await {
                tracing::error!("Failed to render book: {}", e);
            }

            set_progress("export", "Generating PDF and EPUB", 0.90).await;

            let author = request.author_name.as_deref().unwrap_or("AuthorWorks User");
            if let Err(e) = generate_pdf_and_epub(&output_dir, &request.title, author) {
                tracing::error!("Failed to generate PDF/EPUB: {}", e);
            }

            // Collect chapter content + artifacts so callers can sync and download.
            let chapters: Vec<JobChapter> = book
                .chapters
                .iter()
                .map(|c| JobChapter {
                    number: c.number,
                    title: c.title.clone(),
                    content: c.content.clone(),
                })
                .collect();
            let synopsis = book.context.synopsis.content.clone();
            let files = list_book_files(&output_dir);

            update_job(&state, &job_id, |job| {
                job.status = "completed".to_string();
                job.phase = "complete".to_string();
                job.current_step = "Book generation complete".to_string();
                job.progress = 1.0;
                job.output_path = Some(output_dir.to_string_lossy().to_string());
                job.synopsis = if synopsis.is_empty() { None } else { Some(synopsis) };
                job.chapters = Some(chapters);
                job.files = Some(files);
            })
            .await;

            tracing::info!("Book generation completed for job {}", job_id);
        }
        Err(e) => {
            fail_job(format!("Generation failed: {}", e)).await;
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
    let snapshot = {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            if job.status == "running" || job.status == "pending" {
                job.status = "cancelled".to_string();
                job.updated_at = chrono::Utc::now().to_rfc3339();
            }
            Some(job.clone())
        } else {
            None
        }
    };
    match snapshot {
        Some(job) => {
            persist_job(&state.jobs_dir(), &job);
            Ok(Json(job))
        }
        None => Err((StatusCode::NOT_FOUND, "Job not found".to_string())),
    }
}

#[derive(Debug, Serialize)]
struct FileListing {
    book_id: String,
    files: Vec<String>,
}

async fn list_files(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<String>,
) -> Result<Json<FileListing>, (StatusCode, String)> {
    if !is_safe_path_component(&book_id) {
        return Err((StatusCode::BAD_REQUEST, "Invalid book_id".to_string()));
    }
    let book_dir = state.output_base.join(&book_id);
    if !book_dir.is_dir() {
        return Err((StatusCode::NOT_FOUND, "Book output not found".to_string()));
    }
    Ok(Json(FileListing {
        book_id,
        files: list_book_files(&book_dir),
    }))
}

async fn download_file(
    State(state): State<Arc<AppState>>,
    Path((book_id, filename)): Path<(String, String)>,
) -> Result<Response, (StatusCode, String)> {
    if !is_safe_path_component(&book_id) || !is_safe_path_component(&filename) {
        return Err((StatusCode::BAD_REQUEST, "Invalid path".to_string()));
    }
    let file_path = state.output_base.join(&book_id).join(&filename);
    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for(&filename))
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> GenerateRequest {
        GenerateRequest {
            book_id: "test-book".to_string(),
            title: "Test".to_string(),
            braindump: None,
            genre: None,
            style: None,
            characters: None,
            synopsis: None,
            chapter_count: None,
            author_name: None,
            llm_api_base: None,
            llm_model: None,
            llm_api_key: None,
        }
    }

    #[test]
    fn override_switches_provider_and_never_reuses_platform_key() {
        let mut config = Config::default();
        config.llm_provider = "openai".to_string();
        config.openai_api_key = "platform-secret".to_string();

        let mut request = base_request();
        request.llm_api_base = Some("https://api.example.com/v1/".to_string());
        request.llm_model = Some("gpt-4o-mini".to_string());

        apply_llm_overrides(&mut config, &request);

        assert_eq!(config.llm_provider, "openai");
        assert_eq!(config.openai_api_base, "https://api.example.com/v1");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.openai_api_key, "no-key");
    }

    #[test]
    fn override_uses_supplied_key() {
        let mut config = Config::default();
        let mut request = base_request();
        request.llm_api_base = Some("https://llm.example.com/v1".to_string());
        request.llm_api_key = Some("user-key".to_string());

        apply_llm_overrides(&mut config, &request);

        assert_eq!(config.openai_api_key, "user-key");
    }

    #[test]
    fn no_override_keeps_platform_config() {
        let mut config = Config::default();
        config.llm_provider = "openai".to_string();
        config.openai_api_key = "platform-secret".to_string();
        let original_base = config.openai_api_base.clone();

        apply_llm_overrides(&mut config, &base_request());

        assert_eq!(config.llm_provider, "openai");
        assert_eq!(config.openai_api_base, original_base);
        assert_eq!(config.openai_api_key, "platform-secret");
    }

    #[test]
    fn model_only_override_keeps_endpoint() {
        let mut config = Config::default();
        let original_base = config.openai_api_base.clone();
        let mut request = base_request();
        request.llm_model = Some("long".to_string());

        apply_llm_overrides(&mut config, &request);

        assert_eq!(config.model, "long");
        assert_eq!(config.openai_api_base, original_base);
    }

    #[test]
    fn rejects_unsafe_path_components() {
        assert!(!is_safe_path_component(""));
        assert!(!is_safe_path_component(".."));
        assert!(!is_safe_path_component("../etc"));
        assert!(!is_safe_path_component("a/b"));
        assert!(!is_safe_path_component("a\\b"));
        assert!(!is_safe_path_component(".hidden"));
        assert!(is_safe_path_component("book-123_v2"));
        assert!(is_safe_path_component("book.epub"));
    }

    #[test]
    fn content_types_map_by_extension() {
        assert_eq!(content_type_for("book.pdf"), "application/pdf");
        assert_eq!(content_type_for("book.epub"), "application/epub+zip");
        assert_eq!(content_type_for("weird.bin"), "application/octet-stream");
    }

    #[test]
    fn persisted_inflight_jobs_marked_failed_on_restore() {
        let dir = std::env::temp_dir().join(format!("aw-jobs-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let job = JobStatus {
            id: "job-1".to_string(),
            book_id: Some("book-1".to_string()),
            status: "running".to_string(),
            phase: "content".to_string(),
            current_step: "Generating".to_string(),
            progress: 0.5,
            error: None,
            output_path: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            synopsis: None,
            chapters: None,
            files: None,
        };
        persist_job(&dir, &job);

        let restored = load_persisted_jobs(&dir);
        assert_eq!(restored.len(), 1);
        let restored_job = restored.get("job-1").unwrap();
        assert_eq!(restored_job.status, "failed");
        assert!(restored_job.error.as_deref().unwrap_or("").contains("restart"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
