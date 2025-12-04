//! AuthorWorks Media Service
//!
//! Handles media processing jobs, transformations, and delivery.
//! Processing is done asynchronously via the media-worker.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - POST /jobs/image - Create image processing job
//! - POST /jobs/audio - Create audio processing job
//! - POST /jobs/video - Create video processing job
//! - GET /jobs/:id - Get job status
//! - GET /jobs - List user's jobs
//! - DELETE /jobs/:id - Cancel job
//! - GET /transform/image - Get transformed image URL
//! - GET /thumbnails/:file_id - Get or generate thumbnail

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

mod models;
mod error;

use error::ServiceError;
use models::*;

#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();

    let result = match (method, path) {
        // Health
        (Method::Get, "/health") => health_handler(),
        (Method::Get, "/") => service_info(),

        // Jobs
        (Method::Post, "/jobs/image") => create_image_job(&req),
        (Method::Post, "/jobs/audio") => create_audio_job(&req),
        (Method::Post, "/jobs/video") => create_video_job(&req),
        (Method::Get, "/jobs") => list_jobs(&req),
        (Method::Get, path) if path.starts_with("/jobs/") => get_job(&req, path),
        (Method::Delete, path) if path.starts_with("/jobs/") => cancel_job(&req, path),

        // Transformations
        (Method::Get, "/transform/image") => transform_image(&req),
        (Method::Get, path) if path.starts_with("/thumbnails/") => get_thumbnail(&req, path),

        // CORS
        (Method::Options, _) => cors_preflight(),

        _ => Err(ServiceError::NotFound(format!("Route not found: {} {}", method, path))),
    };

    match result {
        Ok(response) => Ok(response),
        Err(e) => Ok(e.into_response()),
    }
}

//=============================================================================
// Configuration
//=============================================================================

fn get_db_connection() -> Result<Connection, ServiceError> {
    let url = variables::get("database_url")
        .map_err(|_| ServiceError::Internal("DATABASE_URL not configured".into()))?;
    Connection::open(&url)
        .map_err(|e| ServiceError::Internal(format!("Database connection failed: {}", e)))
}

fn get_user_id(req: &Request) -> Result<Uuid, ServiceError> {
    let user_id = req.header("X-User-Id")
        .and_then(|h| h.as_str())
        .ok_or_else(|| ServiceError::Unauthorized("Missing user ID".into()))?;
    
    Uuid::parse_str(user_id)
        .map_err(|_| ServiceError::Unauthorized("Invalid user ID".into()))
}

//=============================================================================
// Health & Info
//=============================================================================

fn health_handler() -> Result<Response, ServiceError> {
    let db_status = match get_db_connection() {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    json_response(200, serde_json::json!({
        "status": "healthy",
        "service": "media-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Media Service",
        "version": env!("CARGO_PKG_VERSION"),
        "supported_operations": {
            "image": ["resize", "crop", "compress", "convert", "thumbnail", "cover_generation"],
            "audio": ["convert", "compress", "trim", "normalize", "tts"],
            "video": ["convert", "compress", "thumbnail", "trailer"]
        }
    }))
}

fn cors_preflight() -> Result<Response, ServiceError> {
    Ok(Response::builder()
        .status(204)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-User-Id")
        .header("Access-Control-Max-Age", "86400")
        .body(())
        .build())
}

//=============================================================================
// Image Jobs
//=============================================================================

fn create_image_job(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: ImageJobRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    validate_image_operation(&body.operation)?;

    let job_id = Uuid::new_v4();
    let now = Utc::now();

    let job_data = serde_json::json!({
        "type": "image",
        "operation": body.operation,
        "source_file_id": body.source_file_id,
        "options": body.options
    });

    let insert = "INSERT INTO media.jobs (id, user_id, job_type, status, input, created_at)
                  VALUES ($1, $2, 'image', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(job_data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Queue job for processing (via RabbitMQ in production)
    queue_media_job(&conn, &job_id, "image", &job_data)?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "created_at": now.to_rfc3339()
    }))
}

fn validate_image_operation(operation: &str) -> Result<(), ServiceError> {
    let valid_ops = ["resize", "crop", "compress", "convert", "thumbnail", "cover_generation"];
    if !valid_ops.contains(&operation) {
        return Err(ServiceError::BadRequest(format!(
            "Invalid image operation. Valid operations: {}",
            valid_ops.join(", ")
        )));
    }
    Ok(())
}

//=============================================================================
// Audio Jobs
//=============================================================================

fn create_audio_job(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: AudioJobRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    validate_audio_operation(&body.operation)?;

    let job_id = Uuid::new_v4();
    let now = Utc::now();

    let job_data = serde_json::json!({
        "type": "audio",
        "operation": body.operation,
        "source_file_id": body.source_file_id,
        "options": body.options,
        "text": body.text  // For TTS
    });

    let insert = "INSERT INTO media.jobs (id, user_id, job_type, status, input, created_at)
                  VALUES ($1, $2, 'audio', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(job_data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    queue_media_job(&conn, &job_id, "audio", &job_data)?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "created_at": now.to_rfc3339()
    }))
}

fn validate_audio_operation(operation: &str) -> Result<(), ServiceError> {
    let valid_ops = ["convert", "compress", "trim", "normalize", "tts"];
    if !valid_ops.contains(&operation) {
        return Err(ServiceError::BadRequest(format!(
            "Invalid audio operation. Valid operations: {}",
            valid_ops.join(", ")
        )));
    }
    Ok(())
}

//=============================================================================
// Video Jobs
//=============================================================================

fn create_video_job(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: VideoJobRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    validate_video_operation(&body.operation)?;

    let job_id = Uuid::new_v4();
    let now = Utc::now();

    let job_data = serde_json::json!({
        "type": "video",
        "operation": body.operation,
        "source_file_id": body.source_file_id,
        "options": body.options
    });

    let insert = "INSERT INTO media.jobs (id, user_id, job_type, status, input, created_at)
                  VALUES ($1, $2, 'video', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(job_data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    queue_media_job(&conn, &job_id, "video", &job_data)?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "created_at": now.to_rfc3339()
    }))
}

fn validate_video_operation(operation: &str) -> Result<(), ServiceError> {
    let valid_ops = ["convert", "compress", "thumbnail", "trailer"];
    if !valid_ops.contains(&operation) {
        return Err(ServiceError::BadRequest(format!(
            "Invalid video operation. Valid operations: {}",
            valid_ops.join(", ")
        )));
    }
    Ok(())
}

//=============================================================================
// Job Management
//=============================================================================

fn list_jobs(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT id, job_type, status, input, output, error, created_at, started_at, completed_at
                 FROM media.jobs WHERE user_id = $1
                 ORDER BY created_at DESC LIMIT 50";

    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let jobs: Vec<JobSummary> = rows.rows.iter().map(|row| {
        JobSummary {
            id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
            job_type: String::decode(&row[1]).unwrap_or_default(),
            status: String::decode(&row[2]).unwrap_or_default(),
            created_at: String::decode(&row[6]).unwrap_or_default(),
            started_at: String::decode(&row[7]).ok(),
            completed_at: String::decode(&row[8]).ok(),
        }
    }).collect();

    json_response(200, serde_json::json!({
        "jobs": jobs
    }))
}

fn get_job(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let job_id = extract_id_from_path(path, "/jobs/")?;
    let conn = get_db_connection()?;

    let query = "SELECT id, job_type, status, input, output, error, progress, created_at, started_at, completed_at
                 FROM media.jobs WHERE id = $1 AND user_id = $2";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Job not found".into()));
    }

    let row = &rows.rows[0];
    let job = Job {
        id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
        job_type: String::decode(&row[1]).unwrap_or_default(),
        status: String::decode(&row[2]).unwrap_or_default(),
        input: serde_json::from_str(&String::decode(&row[3]).unwrap_or_else(|_| "{}".into())).unwrap_or_default(),
        output: String::decode(&row[4]).ok().and_then(|s| serde_json::from_str(&s).ok()),
        error: String::decode(&row[5]).ok(),
        progress: i32::decode(&row[6]).ok(),
        created_at: String::decode(&row[7]).unwrap_or_default(),
        started_at: String::decode(&row[8]).ok(),
        completed_at: String::decode(&row[9]).ok(),
    };

    json_response(200, job)
}

fn cancel_job(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let job_id = extract_id_from_path(path, "/jobs/")?;
    let conn = get_db_connection()?;

    // Only cancel pending jobs
    let update = "UPDATE media.jobs SET status = 'cancelled' 
                  WHERE id = $1 AND user_id = $2 AND status = 'pending'";
    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let result = conn.execute(update, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::BadRequest("Job not found or cannot be cancelled".into()));
    }

    json_response(200, serde_json::json!({"cancelled": true}))
}

//=============================================================================
// Transformations
//=============================================================================

fn transform_image(req: &Request) -> Result<Response, ServiceError> {
    let _user_id = get_user_id(req)?;
    
    let file_id = get_query_param(req, "file_id")
        .ok_or_else(|| ServiceError::BadRequest("file_id is required".into()))?;
    let width = get_query_param(req, "w").and_then(|s| s.parse().ok());
    let height = get_query_param(req, "h").and_then(|s| s.parse().ok());
    let quality = get_query_param(req, "q").and_then(|s| s.parse().ok()).unwrap_or(80);
    let format = get_query_param(req, "format").unwrap_or_else(|| "webp".into());

    // Build transformation URL
    // In production, this would use a CDN like Cloudflare Images or imgix
    let transform_params = serde_json::json!({
        "file_id": file_id,
        "width": width,
        "height": height,
        "quality": quality,
        "format": format
    });

    // Generate signed URL for transformed image
    let signed_url = generate_transform_url(&file_id, width, height, quality, &format);

    json_response(200, serde_json::json!({
        "url": signed_url,
        "params": transform_params
    }))
}

fn get_thumbnail(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let file_id = path.strip_prefix("/thumbnails/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;
    
    let conn = get_db_connection()?;

    // Check for existing thumbnail
    let query = "SELECT t.s3_key FROM media.thumbnails t
                 JOIN storage.files f ON t.file_id = f.id
                 WHERE f.id = $1 AND f.user_id = $2";
    
    let params = [
        ParameterValue::Str(file_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if !rows.rows.is_empty() {
        let s3_key = String::decode(&rows.rows[0][0]).unwrap_or_default();
        return json_response(200, serde_json::json!({
            "thumbnail_url": format!("/storage/{}", s3_key),
            "status": "ready"
        }));
    }

    // Queue thumbnail generation
    let job_id = Uuid::new_v4();
    let now = Utc::now();

    let job_data = serde_json::json!({
        "type": "image",
        "operation": "thumbnail",
        "source_file_id": file_id,
        "options": {
            "width": 300,
            "height": 400,
            "fit": "cover"
        }
    });

    let insert = "INSERT INTO media.jobs (id, user_id, job_type, status, input, created_at)
                  VALUES ($1, $2, 'image', 'pending', $3, $4)";

    let insert_params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(job_data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &insert_params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    queue_media_job(&conn, &job_id, "image", &job_data)?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "processing",
        "message": "Thumbnail generation queued"
    }))
}

//=============================================================================
// Helper Functions
//=============================================================================

fn queue_media_job(conn: &Connection, job_id: &Uuid, job_type: &str, data: &serde_json::Value) -> Result<(), ServiceError> {
    // In production, this would publish to RabbitMQ
    // For now, store in a queue table that the worker polls
    let id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO media.job_queue (id, job_id, job_type, data, created_at)
                  VALUES ($1, $2, $3, $4, $5)";

    let params = [
        ParameterValue::Str(id.to_string()),
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(job_type.to_string()),
        ParameterValue::Str(data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Queue insert failed: {}", e)))?;

    Ok(())
}

fn generate_transform_url(file_id: &str, width: Option<i32>, height: Option<i32>, quality: i32, format: &str) -> String {
    let mut params = vec![format!("id={}", file_id)];
    if let Some(w) = width { params.push(format!("w={}", w)); }
    if let Some(h) = height { params.push(format!("h={}", h)); }
    params.push(format!("q={}", quality));
    params.push(format!("f={}", format));
    
    format!("/media/transform?{}", params.join("&"))
}

fn extract_id_from_path(path: &str, prefix: &str) -> Result<Uuid, ServiceError> {
    let id_str = path.strip_prefix(prefix)
        .and_then(|s| s.split('/').next())
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    Uuid::parse_str(id_str)
        .map_err(|_| ServiceError::BadRequest("Invalid UUID".into()))
}

fn get_query_param(req: &Request, name: &str) -> Option<String> {
    let path = req.path();
    let query_start = path.find('?')?;
    let query_str = &path[query_start + 1..];
    
    for pair in query_str.split('&') {
        let mut kv = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
            if key == name {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn json_response<T: Serialize>(status: u16, body: T) -> Result<Response, ServiceError> {
    let json = serde_json::to_string(&body)
        .map_err(|e| ServiceError::Internal(format!("JSON error: {}", e)))?;

    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(json)
        .build())
}

fn parse_json_body<T: for<'de> Deserialize<'de>>(req: &Request) -> Result<T, ServiceError> {
    serde_json::from_slice(req.body())
        .map_err(|e| ServiceError::BadRequest(format!("Invalid JSON: {}", e)))
}
