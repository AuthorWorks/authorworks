//! AuthorWorks Content Service
//!
//! Manages books, chapters, scenes, and AI content generation.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - GET /books - List user's books
//! - POST /books - Create new book
//! - GET /books/:id - Get book details
//! - PUT /books/:id - Update book
//! - DELETE /books/:id - Delete book
//! - GET /books/:id/chapters - List chapters
//! - POST /books/:id/chapters - Create chapter
//! - GET /chapters/:id - Get chapter
//! - PUT /chapters/:id - Update chapter
//! - DELETE /chapters/:id - Delete chapter
//! - POST /generate/outline - Generate book outline
//! - POST /generate/chapter - Generate chapter content
//! - POST /generate/enhance - Enhance existing content

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod models;
mod handlers;
mod error;
mod generation;

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

        // Books CRUD
        (Method::Get, "/books") => list_books(&req),
        (Method::Post, "/books") => create_book(&req),
        (Method::Get, path) if path.starts_with("/books/") && !path.contains("/chapters") => {
            get_book(&req, path)
        }
        (Method::Put, path) if path.starts_with("/books/") && !path.contains("/chapters") => {
            update_book(&req, path)
        }
        (Method::Delete, path) if path.starts_with("/books/") && !path.contains("/chapters") => {
            delete_book(&req, path)
        }

        // Chapters
        (Method::Get, path) if path.ends_with("/chapters") => list_chapters(&req, path),
        (Method::Post, path) if path.ends_with("/chapters") => create_chapter(&req, path),
        (Method::Get, path) if path.starts_with("/chapters/") => get_chapter(&req, path),
        (Method::Put, path) if path.starts_with("/chapters/") => update_chapter(&req, path),
        (Method::Delete, path) if path.starts_with("/chapters/") => delete_chapter(&req, path),

        // Generation
        (Method::Post, "/generate/outline") => generate_outline(&req),
        (Method::Post, "/generate/chapter") => generate_chapter_content(&req),
        (Method::Post, "/generate/enhance") => enhance_content(&req),

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
// Database Connection
//=============================================================================

fn get_db_connection() -> Result<Connection, ServiceError> {
    let url = variables::get("database_url")
        .map_err(|_| ServiceError::Internal("DATABASE_URL not configured".into()))?;
    Connection::open(&url)
        .map_err(|e| ServiceError::Internal(format!("Database connection failed: {}", e)))
}

fn get_user_id(req: &Request) -> Result<Uuid, ServiceError> {
    // Extract user ID from JWT token (set by API gateway after validation)
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
    // Check database connectivity
    let db_status = match get_db_connection() {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    json_response(200, serde_json::json!({
        "status": "healthy",
        "service": "content-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Content Service",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "books": ["GET /books", "POST /books", "GET /books/:id", "PUT /books/:id", "DELETE /books/:id"],
            "chapters": ["GET /books/:id/chapters", "POST /books/:id/chapters", "GET /chapters/:id", "PUT /chapters/:id", "DELETE /chapters/:id"],
            "generation": ["POST /generate/outline", "POST /generate/chapter", "POST /generate/enhance"]
        }
    }))
}

fn cors_preflight() -> Result<Response, ServiceError> {
    Ok(Response::builder()
        .status(204)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-User-Id")
        .header("Access-Control-Max-Age", "86400")
        .body(())
        .build())
}

//=============================================================================
// Books CRUD
//=============================================================================

fn list_books(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT id, title, description, genre, status, cover_image_url, word_count, 
                 created_at, updated_at, published_at 
                 FROM content.books WHERE author_id = $1 ORDER BY updated_at DESC";
    
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let books: Vec<BookSummary> = rows.rows.iter().map(|row| {
        BookSummary {
            id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
            title: String::decode(&row[1]).unwrap_or_default(),
            description: String::decode(&row[2]).ok(),
            genre: String::decode(&row[3]).ok(),
            status: String::decode(&row[4]).unwrap_or_else(|_| "draft".into()),
            cover_image_url: String::decode(&row[5]).ok(),
            word_count: i32::decode(&row[6]).unwrap_or(0),
            created_at: String::decode(&row[7]).unwrap_or_default(),
            updated_at: String::decode(&row[8]).unwrap_or_default(),
        }
    }).collect();

    json_response(200, serde_json::json!({
        "books": books,
        "total": books.len()
    }))
}

fn create_book(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: CreateBookRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    let book_id = Uuid::new_v4();
    let now = Utc::now();

    let query = "INSERT INTO content.books (id, author_id, title, description, genre, status, metadata, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7, $7)
                 RETURNING id";

    let metadata = serde_json::to_string(&body.metadata.unwrap_or_default())
        .unwrap_or_else(|_| "{}".into());

    let params = [
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.title.clone()),
        ParameterValue::Str(body.description.clone().unwrap_or_default()),
        ParameterValue::Str(body.genre.clone().unwrap_or_default()),
        ParameterValue::Str(metadata),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    json_response(201, serde_json::json!({
        "id": book_id,
        "title": body.title,
        "description": body.description,
        "genre": body.genre,
        "status": "draft",
        "created_at": now.to_rfc3339(),
        "message": "Book created successfully"
    }))
}

fn get_book(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let book_id = extract_id_from_path(path, "/books/")?;
    let conn = get_db_connection()?;

    let query = "SELECT id, title, description, genre, status, cover_image_url, word_count,
                 metadata, created_at, updated_at, published_at
                 FROM content.books WHERE id = $1 AND author_id = $2";

    let params = [
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Book not found".into()));
    }

    let row = &rows.rows[0];
    let book = Book {
        id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
        author_id: user_id,
        title: String::decode(&row[1]).unwrap_or_default(),
        description: String::decode(&row[2]).ok(),
        genre: String::decode(&row[3]).ok(),
        status: String::decode(&row[4]).unwrap_or_else(|_| "draft".into()),
        cover_image_url: String::decode(&row[5]).ok(),
        word_count: i32::decode(&row[6]).unwrap_or(0),
        metadata: serde_json::from_str(&String::decode(&row[7]).unwrap_or_else(|_| "{}".into())).unwrap_or_default(),
        created_at: String::decode(&row[8]).unwrap_or_default(),
        updated_at: String::decode(&row[9]).unwrap_or_default(),
        published_at: String::decode(&row[10]).ok(),
    };

    json_response(200, book)
}

fn update_book(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let book_id = extract_id_from_path(path, "/books/")?;
    let body: UpdateBookRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    let now = Utc::now();

    // Build dynamic update query
    let mut updates = vec!["updated_at = $3"];
    let mut param_idx = 4;
    
    if body.title.is_some() { updates.push("title = $4"); param_idx += 1; }
    if body.description.is_some() { updates.push(&format!("description = ${}", param_idx)); param_idx += 1; }
    if body.genre.is_some() { updates.push(&format!("genre = ${}", param_idx)); param_idx += 1; }
    if body.status.is_some() { updates.push(&format!("status = ${}", param_idx)); }

    let query = format!(
        "UPDATE content.books SET {} WHERE id = $1 AND author_id = $2",
        updates.join(", ")
    );

    let mut params: Vec<ParameterValue> = vec![
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    if let Some(ref title) = body.title { params.push(ParameterValue::Str(title.clone())); }
    if let Some(ref desc) = body.description { params.push(ParameterValue::Str(desc.clone())); }
    if let Some(ref genre) = body.genre { params.push(ParameterValue::Str(genre.clone())); }
    if let Some(ref status) = body.status { params.push(ParameterValue::Str(status.clone())); }

    let result = conn.execute(&query, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::NotFound("Book not found".into()));
    }

    json_response(200, serde_json::json!({
        "message": "Book updated successfully",
        "updated_at": now.to_rfc3339()
    }))
}

fn delete_book(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let book_id = extract_id_from_path(path, "/books/")?;
    let conn = get_db_connection()?;

    let query = "DELETE FROM content.books WHERE id = $1 AND author_id = $2";
    let params = [
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let result = conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Delete failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::NotFound("Book not found".into()));
    }

    json_response(200, serde_json::json!({
        "message": "Book deleted successfully"
    }))
}

//=============================================================================
// Chapters CRUD
//=============================================================================

fn list_chapters(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let book_id = extract_id_from_path(path, "/books/")?;
    let conn = get_db_connection()?;

    // Verify book ownership
    verify_book_ownership(&conn, &book_id, &user_id)?;

    let query = "SELECT id, title, chapter_number, word_count, status, created_at, updated_at
                 FROM content.chapters WHERE book_id = $1 ORDER BY chapter_number ASC";

    let params = [ParameterValue::Str(book_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let chapters: Vec<ChapterSummary> = rows.rows.iter().map(|row| {
        ChapterSummary {
            id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
            title: String::decode(&row[1]).unwrap_or_default(),
            chapter_number: i32::decode(&row[2]).unwrap_or(0),
            word_count: i32::decode(&row[3]).unwrap_or(0),
            status: String::decode(&row[4]).unwrap_or_else(|_| "draft".into()),
            created_at: String::decode(&row[5]).unwrap_or_default(),
            updated_at: String::decode(&row[6]).unwrap_or_default(),
        }
    }).collect();

    json_response(200, serde_json::json!({
        "chapters": chapters,
        "total": chapters.len()
    }))
}

fn create_chapter(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let book_id = extract_id_from_path(path, "/books/")?;
    let body: CreateChapterRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_book_ownership(&conn, &book_id, &user_id)?;

    let chapter_id = Uuid::new_v4();
    let now = Utc::now();
    let word_count = body.content.as_ref().map(|c| c.split_whitespace().count() as i32).unwrap_or(0);

    let query = "INSERT INTO content.chapters (id, book_id, title, content, chapter_number, word_count, status, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, 'draft', $7, $7)";

    let params = [
        ParameterValue::Str(chapter_id.to_string()),
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(body.title.clone()),
        ParameterValue::Str(body.content.clone().unwrap_or_default()),
        ParameterValue::Int32(body.chapter_number),
        ParameterValue::Int32(word_count),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Update book word count
    update_book_word_count(&conn, &book_id)?;

    json_response(201, serde_json::json!({
        "id": chapter_id,
        "title": body.title,
        "chapter_number": body.chapter_number,
        "word_count": word_count,
        "status": "draft",
        "created_at": now.to_rfc3339()
    }))
}

fn get_chapter(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let chapter_id = extract_id_from_path(path, "/chapters/")?;
    let conn = get_db_connection()?;

    let query = "SELECT c.id, c.book_id, c.title, c.content, c.chapter_number, c.word_count, 
                 c.status, c.created_at, c.updated_at
                 FROM content.chapters c
                 JOIN content.books b ON c.book_id = b.id
                 WHERE c.id = $1 AND b.author_id = $2";

    let params = [
        ParameterValue::Str(chapter_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Chapter not found".into()));
    }

    let row = &rows.rows[0];
    let chapter = Chapter {
        id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
        book_id: Uuid::parse_str(&String::decode(&row[1]).unwrap_or_default()).unwrap_or_default(),
        title: String::decode(&row[2]).unwrap_or_default(),
        content: String::decode(&row[3]).ok(),
        chapter_number: i32::decode(&row[4]).unwrap_or(0),
        word_count: i32::decode(&row[5]).unwrap_or(0),
        status: String::decode(&row[6]).unwrap_or_else(|_| "draft".into()),
        created_at: String::decode(&row[7]).unwrap_or_default(),
        updated_at: String::decode(&row[8]).unwrap_or_default(),
    };

    json_response(200, chapter)
}

fn update_chapter(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let chapter_id = extract_id_from_path(path, "/chapters/")?;
    let body: UpdateChapterRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    // Get book_id for ownership check and word count update
    let book_id = get_chapter_book_id(&conn, &chapter_id, &user_id)?;

    let now = Utc::now();
    let word_count = body.content.as_ref().map(|c| c.split_whitespace().count() as i32);

    let query = "UPDATE content.chapters SET 
                 title = COALESCE($3, title),
                 content = COALESCE($4, content),
                 word_count = COALESCE($5, word_count),
                 status = COALESCE($6, status),
                 updated_at = $7
                 WHERE id = $1";

    let params = [
        ParameterValue::Str(chapter_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.title.unwrap_or_default()),
        ParameterValue::Str(body.content.unwrap_or_default()),
        ParameterValue::Int32(word_count.unwrap_or(0)),
        ParameterValue::Str(body.status.unwrap_or_default()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    // Update book word count
    update_book_word_count(&conn, &book_id)?;

    json_response(200, serde_json::json!({
        "message": "Chapter updated successfully",
        "updated_at": now.to_rfc3339()
    }))
}

fn delete_chapter(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let chapter_id = extract_id_from_path(path, "/chapters/")?;
    let conn = get_db_connection()?;

    let book_id = get_chapter_book_id(&conn, &chapter_id, &user_id)?;

    let query = "DELETE FROM content.chapters WHERE id = $1";
    let params = [ParameterValue::Str(chapter_id.to_string())];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Delete failed: {}", e)))?;

    update_book_word_count(&conn, &book_id)?;

    json_response(200, serde_json::json!({
        "message": "Chapter deleted successfully"
    }))
}

//=============================================================================
// Content Generation
//=============================================================================

fn generate_outline(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: GenerateOutlineRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_book_ownership(&conn, &body.book_id, &user_id)?;

    // Queue the generation job via RabbitMQ
    let job_id = Uuid::new_v4();
    let job = serde_json::json!({
        "type": "GenerateOutline",
        "job_id": job_id,
        "book_id": body.book_id,
        "prompt": body.prompt,
        "genre": body.genre,
        "style": body.style,
        "chapter_count": body.chapter_count.unwrap_or(10)
    });

    // In production, this would publish to RabbitMQ
    // For now, store in a jobs table
    let query = "INSERT INTO content.generation_jobs (id, book_id, job_type, status, input, created_at)
                 VALUES ($1, $2, 'outline', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(body.book_id.to_string()),
        ParameterValue::Str(job.to_string()),
        ParameterValue::Str(Utc::now().to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Job creation failed: {}", e)))?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "message": "Outline generation queued",
        "check_status": format!("/jobs/{}", job_id)
    }))
}

fn generate_chapter_content(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: GenerateChapterRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    let book_id = get_chapter_book_id(&conn, &body.chapter_id, &user_id)?;

    let job_id = Uuid::new_v4();
    let job = serde_json::json!({
        "type": "GenerateChapter",
        "job_id": job_id,
        "book_id": book_id,
        "chapter_id": body.chapter_id,
        "outline": body.outline,
        "context": body.context,
        "style": body.style
    });

    let query = "INSERT INTO content.generation_jobs (id, book_id, job_type, status, input, created_at)
                 VALUES ($1, $2, 'chapter', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(job.to_string()),
        ParameterValue::Str(Utc::now().to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Job creation failed: {}", e)))?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "message": "Chapter generation queued"
    }))
}

fn enhance_content(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: EnhanceContentRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    let book_id = get_chapter_book_id(&conn, &body.chapter_id, &user_id)?;

    let job_id = Uuid::new_v4();
    let job = serde_json::json!({
        "type": "EnhanceContent",
        "job_id": job_id,
        "book_id": book_id,
        "chapter_id": body.chapter_id,
        "content": body.content,
        "enhancement_type": body.enhancement_type,
        "instructions": body.instructions
    });

    let query = "INSERT INTO content.generation_jobs (id, book_id, job_type, status, input, created_at)
                 VALUES ($1, $2, 'enhance', 'pending', $3, $4)";

    let params = [
        ParameterValue::Str(job_id.to_string()),
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(job.to_string()),
        ParameterValue::Str(Utc::now().to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Job creation failed: {}", e)))?;

    json_response(202, serde_json::json!({
        "job_id": job_id,
        "status": "pending",
        "message": "Content enhancement queued"
    }))
}

//=============================================================================
// Helper Functions
//=============================================================================

fn verify_book_ownership(conn: &Connection, book_id: &Uuid, user_id: &Uuid) -> Result<(), ServiceError> {
    let query = "SELECT 1 FROM content.books WHERE id = $1 AND author_id = $2";
    let params = [
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Book not found".into()));
    }
    Ok(())
}

fn get_chapter_book_id(conn: &Connection, chapter_id: &Uuid, user_id: &Uuid) -> Result<Uuid, ServiceError> {
    let query = "SELECT c.book_id FROM content.chapters c
                 JOIN content.books b ON c.book_id = b.id
                 WHERE c.id = $1 AND b.author_id = $2";
    
    let params = [
        ParameterValue::Str(chapter_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Chapter not found".into()));
    }

    let book_id_str = String::decode(&rows.rows[0][0])
        .map_err(|_| ServiceError::Internal("Failed to decode book_id".into()))?;
    
    Uuid::parse_str(&book_id_str)
        .map_err(|_| ServiceError::Internal("Invalid book_id".into()))
}

fn update_book_word_count(conn: &Connection, book_id: &Uuid) -> Result<(), ServiceError> {
    let query = "UPDATE content.books SET word_count = (
                     SELECT COALESCE(SUM(word_count), 0) FROM content.chapters WHERE book_id = $1
                 ), updated_at = $2 WHERE id = $1";

    let params = [
        ParameterValue::Str(book_id.to_string()),
        ParameterValue::Str(Utc::now().to_rfc3339()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;
    Ok(())
}

fn extract_id_from_path(path: &str, prefix: &str) -> Result<Uuid, ServiceError> {
    let id_str = path.strip_prefix(prefix)
        .and_then(|s| s.split('/').next())
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    Uuid::parse_str(id_str)
        .map_err(|_| ServiceError::BadRequest("Invalid UUID".into()))
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
