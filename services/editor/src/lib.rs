//! AuthorWorks Editor Service
//!
//! Provides collaborative editing with operational transformation,
//! document versioning, and real-time sync support.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - GET /documents/:id - Get document state
//! - POST /documents/:id/operations - Submit edit operation
//! - GET /documents/:id/history - Get edit history
//! - POST /documents/:id/checkpoint - Create checkpoint
//! - GET /documents/:id/checkpoints - List checkpoints
//! - POST /documents/:id/revert - Revert to checkpoint
//! - GET /documents/:id/presence - Get active collaborators
//! - POST /documents/:id/presence - Update presence
//! - GET /documents/:id/comments - Get comments
//! - POST /documents/:id/comments - Add comment
//! - DELETE /comments/:id - Delete comment

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

mod models;
mod error;
mod ot;

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

        // Document state
        (Method::Get, path) if path.starts_with("/documents/") && !path.contains('/') => {
            get_document(&req, path)
        }

        // Operations
        (Method::Post, path) if path.ends_with("/operations") => submit_operation(&req, path),
        (Method::Get, path) if path.ends_with("/history") => get_history(&req, path),

        // Checkpoints
        (Method::Post, path) if path.ends_with("/checkpoint") => create_checkpoint(&req, path),
        (Method::Get, path) if path.ends_with("/checkpoints") => list_checkpoints(&req, path),
        (Method::Post, path) if path.ends_with("/revert") => revert_to_checkpoint(&req, path),

        // Presence
        (Method::Get, path) if path.ends_with("/presence") => get_presence(&req, path),
        (Method::Post, path) if path.ends_with("/presence") => update_presence(&req, path),

        // Comments
        (Method::Get, path) if path.ends_with("/comments") => get_comments(&req, path),
        (Method::Post, path) if path.ends_with("/comments") => add_comment(&req, path),
        (Method::Delete, path) if path.starts_with("/comments/") => delete_comment(&req, path),

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
        "service": "editor-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Editor Service",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["operational-transformation", "real-time-sync", "versioning", "comments"]
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
// Document Operations
//=============================================================================

fn get_document(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id(path)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let query = "SELECT d.id, d.content, d.version, d.updated_at,
                 (SELECT COUNT(*) FROM editor.operations WHERE document_id = d.id) as op_count
                 FROM editor.documents d WHERE d.id = $1";

    let params = [ParameterValue::Str(document_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        // Create new document state
        let now = Utc::now();
        let insert = "INSERT INTO editor.documents (id, content, version, created_at, updated_at)
                      VALUES ($1, '', 0, $2, $2)";
        let insert_params = [
            ParameterValue::Str(document_id.to_string()),
            ParameterValue::Str(now.to_rfc3339()),
        ];
        conn.execute(insert, &insert_params)
            .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

        return json_response(200, serde_json::json!({
            "id": document_id,
            "content": "",
            "version": 0,
            "operations": 0,
            "updated_at": now.to_rfc3339()
        }));
    }

    let row = &rows.rows[0];
    json_response(200, serde_json::json!({
        "id": document_id,
        "content": String::decode(&row[1]).unwrap_or_default(),
        "version": i64::decode(&row[2]).unwrap_or(0),
        "operations": i64::decode(&row[4]).unwrap_or(0),
        "updated_at": String::decode(&row[3]).unwrap_or_default()
    }))
}

fn submit_operation(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/operations")?;
    let body: OperationRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    // Get current document state
    let doc_query = "SELECT content, version FROM editor.documents WHERE id = $1 FOR UPDATE";
    let doc_params = [ParameterValue::Str(document_id.to_string())];
    let doc_rows = conn.query(doc_query, &doc_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let (current_content, current_version) = if doc_rows.rows.is_empty() {
        ("".to_string(), 0i64)
    } else {
        (
            String::decode(&doc_rows.rows[0][0]).unwrap_or_default(),
            i64::decode(&doc_rows.rows[0][1]).unwrap_or(0),
        )
    };

    // Check for version conflict
    if body.base_version != current_version {
        // Need to transform operation against concurrent operations
        let ops_query = "SELECT operation FROM editor.operations 
                         WHERE document_id = $1 AND version > $2 ORDER BY version ASC";
        let ops_params = [
            ParameterValue::Str(document_id.to_string()),
            ParameterValue::Int64(body.base_version),
        ];
        let ops_rows = conn.query(ops_query, &ops_params)
            .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

        // Transform against concurrent operations
        let mut transformed_op = body.operation.clone();
        for op_row in &ops_rows.rows {
            let concurrent_op: Operation = serde_json::from_str(
                &String::decode(&op_row[0]).unwrap_or_default()
            ).map_err(|e| ServiceError::Internal(format!("Parse failed: {}", e)))?;
            transformed_op = transform_operation(&transformed_op, &concurrent_op);
        }

        // Apply transformed operation
        let new_content = apply_operation(&current_content, &transformed_op)?;
        let new_version = current_version + 1;
        let now = Utc::now();

        // Store operation
        let op_id = Uuid::new_v4();
        let op_insert = "INSERT INTO editor.operations (id, document_id, user_id, version, operation, created_at)
                         VALUES ($1, $2, $3, $4, $5, $6)";
        let op_params = [
            ParameterValue::Str(op_id.to_string()),
            ParameterValue::Str(document_id.to_string()),
            ParameterValue::Str(user_id.to_string()),
            ParameterValue::Int64(new_version),
            ParameterValue::Str(serde_json::to_string(&transformed_op).unwrap_or_default()),
            ParameterValue::Str(now.to_rfc3339()),
        ];
        conn.execute(op_insert, &op_params)
            .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

        // Update document
        let doc_update = "UPDATE editor.documents SET content = $2, version = $3, updated_at = $4 WHERE id = $1";
        let update_params = [
            ParameterValue::Str(document_id.to_string()),
            ParameterValue::Str(new_content.clone()),
            ParameterValue::Int64(new_version),
            ParameterValue::Str(now.to_rfc3339()),
        ];
        conn.execute(doc_update, &update_params)
            .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

        return json_response(200, serde_json::json!({
            "version": new_version,
            "transformed_operation": transformed_op,
            "content": new_content
        }));
    }

    // No conflict - apply directly
    let new_content = apply_operation(&current_content, &body.operation)?;
    let new_version = current_version + 1;
    let now = Utc::now();

    // Store operation
    let op_id = Uuid::new_v4();
    let op_insert = "INSERT INTO editor.operations (id, document_id, user_id, version, operation, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)";
    let op_params = [
        ParameterValue::Str(op_id.to_string()),
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Int64(new_version),
        ParameterValue::Str(serde_json::to_string(&body.operation).unwrap_or_default()),
        ParameterValue::Str(now.to_rfc3339()),
    ];
    conn.execute(op_insert, &op_params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Update or insert document
    let doc_upsert = "INSERT INTO editor.documents (id, content, version, created_at, updated_at)
                      VALUES ($1, $2, $3, $4, $4)
                      ON CONFLICT (id) DO UPDATE SET content = $2, version = $3, updated_at = $4";
    let doc_params = [
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(new_content.clone()),
        ParameterValue::Int64(new_version),
        ParameterValue::Str(now.to_rfc3339()),
    ];
    conn.execute(doc_upsert, &doc_params)
        .map_err(|e| ServiceError::Internal(format!("Upsert failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "version": new_version,
        "operation": body.operation,
        "content": new_content
    }))
}

fn get_history(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/history")?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let query = "SELECT o.id, o.user_id, o.version, o.operation, o.created_at, u.name
                 FROM editor.operations o
                 LEFT JOIN users.users u ON o.user_id = u.id
                 WHERE o.document_id = $1 ORDER BY o.version DESC LIMIT 100";

    let params = [ParameterValue::Str(document_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let history: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "user_id": String::decode(&row[1]).unwrap_or_default(),
            "user_name": String::decode(&row[5]).ok(),
            "version": i64::decode(&row[2]).unwrap_or(0),
            "operation": serde_json::from_str::<serde_json::Value>(
                &String::decode(&row[3]).unwrap_or_else(|_| "{}".into())
            ).unwrap_or_default(),
            "created_at": String::decode(&row[4]).unwrap_or_default()
        })
    }).collect();

    json_response(200, serde_json::json!({
        "history": history,
        "total": history.len()
    }))
}

//=============================================================================
// Checkpoints
//=============================================================================

fn create_checkpoint(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/checkpoint")?;
    let body: CheckpointRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    // Get current document state
    let doc_query = "SELECT content, version FROM editor.documents WHERE id = $1";
    let doc_params = [ParameterValue::Str(document_id.to_string())];
    let doc_rows = conn.query(doc_query, &doc_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if doc_rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Document not found".into()));
    }

    let content = String::decode(&doc_rows.rows[0][0]).unwrap_or_default();
    let version = i64::decode(&doc_rows.rows[0][1]).unwrap_or(0);

    let checkpoint_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO editor.checkpoints (id, document_id, user_id, name, content, version, created_at)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)";
    let params = [
        ParameterValue::Str(checkpoint_id.to_string()),
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.name.clone()),
        ParameterValue::Str(content),
        ParameterValue::Int64(version),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    json_response(201, serde_json::json!({
        "id": checkpoint_id,
        "name": body.name,
        "version": version,
        "created_at": now.to_rfc3339()
    }))
}

fn list_checkpoints(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/checkpoints")?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let query = "SELECT c.id, c.name, c.version, c.created_at, u.name as user_name
                 FROM editor.checkpoints c
                 LEFT JOIN users.users u ON c.user_id = u.id
                 WHERE c.document_id = $1 ORDER BY c.created_at DESC";

    let params = [ParameterValue::Str(document_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let checkpoints: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "name": String::decode(&row[1]).unwrap_or_default(),
            "version": i64::decode(&row[2]).unwrap_or(0),
            "created_at": String::decode(&row[3]).unwrap_or_default(),
            "created_by": String::decode(&row[4]).ok()
        })
    }).collect();

    json_response(200, serde_json::json!({
        "checkpoints": checkpoints
    }))
}

fn revert_to_checkpoint(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/revert")?;
    let body: RevertRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    // Get checkpoint
    let cp_query = "SELECT content, version FROM editor.checkpoints WHERE id = $1 AND document_id = $2";
    let cp_params = [
        ParameterValue::Str(body.checkpoint_id.to_string()),
        ParameterValue::Str(document_id.to_string()),
    ];
    let cp_rows = conn.query(cp_query, &cp_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if cp_rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Checkpoint not found".into()));
    }

    let content = String::decode(&cp_rows.rows[0][0]).unwrap_or_default();
    let now = Utc::now();

    // Get current version and increment
    let doc_query = "SELECT version FROM editor.documents WHERE id = $1";
    let doc_params = [ParameterValue::Str(document_id.to_string())];
    let doc_rows = conn.query(doc_query, &doc_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let current_version = if doc_rows.rows.is_empty() {
        0i64
    } else {
        i64::decode(&doc_rows.rows[0][0]).unwrap_or(0)
    };
    let new_version = current_version + 1;

    // Update document
    let update = "UPDATE editor.documents SET content = $2, version = $3, updated_at = $4 WHERE id = $1";
    let update_params = [
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(content.clone()),
        ParameterValue::Int64(new_version),
        ParameterValue::Str(now.to_rfc3339()),
    ];
    conn.execute(update, &update_params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    // Record revert operation
    let op_id = Uuid::new_v4();
    let revert_op = Operation::Revert { checkpoint_id: body.checkpoint_id };
    let op_insert = "INSERT INTO editor.operations (id, document_id, user_id, version, operation, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)";
    let op_params = [
        ParameterValue::Str(op_id.to_string()),
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Int64(new_version),
        ParameterValue::Str(serde_json::to_string(&revert_op).unwrap_or_default()),
        ParameterValue::Str(now.to_rfc3339()),
    ];
    conn.execute(op_insert, &op_params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "version": new_version,
        "content": content,
        "reverted_at": now.to_rfc3339()
    }))
}

//=============================================================================
// Presence
//=============================================================================

fn get_presence(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/presence")?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    // Get active users (updated in last 30 seconds)
    let query = "SELECT p.user_id, p.cursor_position, p.selection_start, p.selection_end, 
                 p.updated_at, u.name, u.avatar_url
                 FROM editor.presence p
                 LEFT JOIN users.users u ON p.user_id = u.id
                 WHERE p.document_id = $1 AND p.updated_at > NOW() - INTERVAL '30 seconds'";

    let params = [ParameterValue::Str(document_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let presence: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "user_id": String::decode(&row[0]).unwrap_or_default(),
            "cursor_position": i32::decode(&row[1]).ok(),
            "selection": {
                "start": i32::decode(&row[2]).ok(),
                "end": i32::decode(&row[3]).ok()
            },
            "updated_at": String::decode(&row[4]).unwrap_or_default(),
            "user": {
                "name": String::decode(&row[5]).ok(),
                "avatar_url": String::decode(&row[6]).ok()
            }
        })
    }).collect();

    json_response(200, serde_json::json!({
        "presence": presence,
        "active_users": presence.len()
    }))
}

fn update_presence(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/presence")?;
    let body: PresenceUpdate = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let now = Utc::now();
    let upsert = "INSERT INTO editor.presence (document_id, user_id, cursor_position, selection_start, selection_end, updated_at)
                  VALUES ($1, $2, $3, $4, $5, $6)
                  ON CONFLICT (document_id, user_id) DO UPDATE SET 
                  cursor_position = $3, selection_start = $4, selection_end = $5, updated_at = $6";

    let params = [
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Int32(body.cursor_position.unwrap_or(0)),
        ParameterValue::Int32(body.selection.as_ref().map(|s| s.start).unwrap_or(0)),
        ParameterValue::Int32(body.selection.as_ref().map(|s| s.end).unwrap_or(0)),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(upsert, &params)
        .map_err(|e| ServiceError::Internal(format!("Upsert failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "updated_at": now.to_rfc3339()
    }))
}

//=============================================================================
// Comments
//=============================================================================

fn get_comments(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/comments")?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let query = "SELECT c.id, c.user_id, c.content, c.position_start, c.position_end, 
                 c.resolved, c.created_at, u.name, u.avatar_url
                 FROM editor.comments c
                 LEFT JOIN users.users u ON c.user_id = u.id
                 WHERE c.document_id = $1 ORDER BY c.position_start ASC";

    let params = [ParameterValue::Str(document_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let comments: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "user_id": String::decode(&row[1]).unwrap_or_default(),
            "content": String::decode(&row[2]).unwrap_or_default(),
            "position": {
                "start": i32::decode(&row[3]).unwrap_or(0),
                "end": i32::decode(&row[4]).unwrap_or(0)
            },
            "resolved": bool::decode(&row[5]).unwrap_or(false),
            "created_at": String::decode(&row[6]).unwrap_or_default(),
            "user": {
                "name": String::decode(&row[7]).ok(),
                "avatar_url": String::decode(&row[8]).ok()
            }
        })
    }).collect();

    json_response(200, serde_json::json!({
        "comments": comments
    }))
}

fn add_comment(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let document_id = extract_document_id_from_sub_path(path, "/comments")?;
    let body: CommentRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    verify_document_access(&conn, &document_id, &user_id)?;

    let comment_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO editor.comments (id, document_id, user_id, content, position_start, position_end, created_at)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)";

    let params = [
        ParameterValue::Str(comment_id.to_string()),
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.content.clone()),
        ParameterValue::Int32(body.position.start),
        ParameterValue::Int32(body.position.end),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    json_response(201, serde_json::json!({
        "id": comment_id,
        "content": body.content,
        "position": body.position,
        "created_at": now.to_rfc3339()
    }))
}

fn delete_comment(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let comment_id = extract_id_from_path(path, "/comments/")?;
    let conn = get_db_connection()?;

    // Verify ownership
    let query = "SELECT document_id FROM editor.comments WHERE id = $1 AND user_id = $2";
    let params = [
        ParameterValue::Str(comment_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Comment not found".into()));
    }

    let delete = "DELETE FROM editor.comments WHERE id = $1";
    let delete_params = [ParameterValue::Str(comment_id.to_string())];
    conn.execute(delete, &delete_params)
        .map_err(|e| ServiceError::Internal(format!("Delete failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "message": "Comment deleted"
    }))
}

//=============================================================================
// Operational Transformation
//=============================================================================

fn transform_operation(op: &Operation, against: &Operation) -> Operation {
    match (op, against) {
        (Operation::Insert { position, text }, Operation::Insert { position: other_pos, text: other_text }) => {
            let new_pos = if *position >= *other_pos {
                position + other_text.len() as i32
            } else {
                *position
            };
            Operation::Insert { position: new_pos, text: text.clone() }
        }
        (Operation::Insert { position, text }, Operation::Delete { position: other_pos, length }) => {
            let new_pos = if *position > *other_pos {
                let adjustment = (*length).min((*position - *other_pos).max(0));
                position - adjustment
            } else {
                *position
            };
            Operation::Insert { position: new_pos, text: text.clone() }
        }
        (Operation::Delete { position, length }, Operation::Insert { position: other_pos, text }) => {
            let new_pos = if *position >= *other_pos {
                position + text.len() as i32
            } else {
                *position
            };
            Operation::Delete { position: new_pos, length: *length }
        }
        (Operation::Delete { position, length }, Operation::Delete { position: other_pos, length: other_length }) => {
            if *position >= other_pos + other_length {
                // Our delete is after theirs
                Operation::Delete { position: position - other_length, length: *length }
            } else if position + length <= *other_pos {
                // Our delete is before theirs
                Operation::Delete { position: *position, length: *length }
            } else {
                // Overlapping deletes - complex case
                let overlap_start = (*position).max(*other_pos);
                let overlap_end = (position + length).min(other_pos + other_length);
                let overlap = (overlap_end - overlap_start).max(0);
                let new_length = length - overlap;
                let new_pos = (*position).min(*other_pos);
                Operation::Delete { position: new_pos, length: new_length }
            }
        }
        _ => op.clone(),
    }
}

fn apply_operation(content: &str, op: &Operation) -> Result<String, ServiceError> {
    match op {
        Operation::Insert { position, text } => {
            let pos = *position as usize;
            if pos > content.len() {
                return Err(ServiceError::BadRequest("Position out of bounds".into()));
            }
            let mut new_content = content.to_string();
            new_content.insert_str(pos, text);
            Ok(new_content)
        }
        Operation::Delete { position, length } => {
            let pos = *position as usize;
            let len = *length as usize;
            if pos + len > content.len() {
                return Err(ServiceError::BadRequest("Delete range out of bounds".into()));
            }
            let mut new_content = content.to_string();
            new_content.replace_range(pos..pos + len, "");
            Ok(new_content)
        }
        Operation::Replace { position, length, text } => {
            let pos = *position as usize;
            let len = *length as usize;
            if pos + len > content.len() {
                return Err(ServiceError::BadRequest("Replace range out of bounds".into()));
            }
            let mut new_content = content.to_string();
            new_content.replace_range(pos..pos + len, text);
            Ok(new_content)
        }
        Operation::Revert { .. } => {
            // Revert is handled specially
            Ok(content.to_string())
        }
    }
}

//=============================================================================
// Helper Functions
//=============================================================================

fn verify_document_access(conn: &Connection, document_id: &Uuid, user_id: &Uuid) -> Result<(), ServiceError> {
    // Document IDs correspond to chapter IDs - verify ownership through book
    let query = "SELECT 1 FROM content.chapters c
                 JOIN content.books b ON c.book_id = b.id
                 WHERE c.id = $1 AND b.author_id = $2";
    let params = [
        ParameterValue::Str(document_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::Forbidden("Access denied".into()));
    }
    Ok(())
}

fn extract_document_id(path: &str) -> Result<Uuid, ServiceError> {
    let id_str = path.strip_prefix("/documents/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    Uuid::parse_str(id_str)
        .map_err(|_| ServiceError::BadRequest("Invalid UUID".into()))
}

fn extract_document_id_from_sub_path(path: &str, suffix: &str) -> Result<Uuid, ServiceError> {
    let without_suffix = path.strip_suffix(suffix)
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;
    
    let id_str = without_suffix.strip_prefix("/documents/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    Uuid::parse_str(id_str)
        .map_err(|_| ServiceError::BadRequest("Invalid UUID".into()))
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
