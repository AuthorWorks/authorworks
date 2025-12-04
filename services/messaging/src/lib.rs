//! AuthorWorks Messaging Service
//!
//! Handles notifications, in-app messaging, and real-time events.
//! Works with frontend WebSocket connections and RabbitMQ for async events.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - GET /notifications - List user notifications
//! - POST /notifications - Create notification (admin)
//! - PUT /notifications/:id/read - Mark as read
//! - DELETE /notifications/:id - Delete notification
//! - GET /messages - List conversations
//! - GET /messages/:conversation_id - Get conversation messages
//! - POST /messages - Send message
//! - DELETE /messages/:id - Delete message
//! - POST /events - Publish event to queue
//! - GET /events/subscribe - SSE endpoint for real-time events

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

        // Notifications
        (Method::Get, "/notifications") => list_notifications(&req),
        (Method::Post, "/notifications") => create_notification(&req),
        (Method::Put, path) if path.ends_with("/read") => mark_notification_read(&req, path),
        (Method::Delete, path) if path.starts_with("/notifications/") => delete_notification(&req, path),
        (Method::Post, "/notifications/read-all") => mark_all_read(&req),

        // Messages
        (Method::Get, "/messages") => list_conversations(&req),
        (Method::Get, path) if path.starts_with("/messages/") => get_conversation(&req, path),
        (Method::Post, "/messages") => send_message(&req),
        (Method::Delete, path) if path.starts_with("/messages/") && !path.contains('/') => {
            delete_message(&req, path)
        }

        // Events
        (Method::Post, "/events") => publish_event(&req),
        (Method::Get, "/events/subscribe") => subscribe_events(&req),

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
        "service": "messaging-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Messaging Service",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["notifications", "direct-messages", "real-time-events"]
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
// Notifications
//=============================================================================

fn list_notifications(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT id, type, title, body, data, read, created_at
                 FROM messaging.notifications
                 WHERE user_id = $1
                 ORDER BY created_at DESC LIMIT 50";

    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let notifications: Vec<Notification> = rows.rows.iter().map(|row| {
        Notification {
            id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
            notification_type: String::decode(&row[1]).unwrap_or_default(),
            title: String::decode(&row[2]).unwrap_or_default(),
            body: String::decode(&row[3]).unwrap_or_default(),
            data: serde_json::from_str(&String::decode(&row[4]).unwrap_or_else(|_| "{}".into())).unwrap_or_default(),
            read: bool::decode(&row[5]).unwrap_or(false),
            created_at: String::decode(&row[6]).unwrap_or_default(),
        }
    }).collect();

    // Count unread
    let unread_query = "SELECT COUNT(*) FROM messaging.notifications WHERE user_id = $1 AND read = false";
    let unread_rows = conn.query(unread_query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;
    let unread_count = if !unread_rows.rows.is_empty() {
        i64::decode(&unread_rows.rows[0][0]).unwrap_or(0)
    } else {
        0
    };

    json_response(200, serde_json::json!({
        "notifications": notifications,
        "unread_count": unread_count
    }))
}

fn create_notification(req: &Request) -> Result<Response, ServiceError> {
    let body: CreateNotificationRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    let notification_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO messaging.notifications 
                  (id, user_id, type, title, body, data, created_at)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)";

    let params = [
        ParameterValue::Str(notification_id.to_string()),
        ParameterValue::Str(body.user_id.to_string()),
        ParameterValue::Str(body.notification_type.clone()),
        ParameterValue::Str(body.title.clone()),
        ParameterValue::Str(body.body.clone()),
        ParameterValue::Str(serde_json::to_string(&body.data).unwrap_or_else(|_| "{}".into())),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Queue real-time event for SSE/WebSocket delivery
    queue_event(&conn, &body.user_id, "notification", serde_json::json!({
        "id": notification_id,
        "type": body.notification_type,
        "title": body.title,
        "body": body.body
    }))?;

    json_response(201, serde_json::json!({
        "id": notification_id,
        "created_at": now.to_rfc3339()
    }))
}

fn mark_notification_read(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let notification_id = extract_id_from_path_with_suffix(path, "/notifications/", "/read")?;
    let conn = get_db_connection()?;

    let update = "UPDATE messaging.notifications SET read = true WHERE id = $1 AND user_id = $2";
    let params = [
        ParameterValue::Str(notification_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let result = conn.execute(update, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::NotFound("Notification not found".into()));
    }

    json_response(200, serde_json::json!({"read": true}))
}

fn mark_all_read(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let update = "UPDATE messaging.notifications SET read = true WHERE user_id = $1 AND read = false";
    let params = [ParameterValue::Str(user_id.to_string())];

    let count = conn.execute(update, &params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "marked_read": count
    }))
}

fn delete_notification(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let notification_id = extract_id_from_path(path, "/notifications/")?;
    let conn = get_db_connection()?;

    let delete = "DELETE FROM messaging.notifications WHERE id = $1 AND user_id = $2";
    let params = [
        ParameterValue::Str(notification_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let result = conn.execute(delete, &params)
        .map_err(|e| ServiceError::Internal(format!("Delete failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::NotFound("Notification not found".into()));
    }

    json_response(200, serde_json::json!({"deleted": true}))
}

//=============================================================================
// Messages
//=============================================================================

fn list_conversations(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT DISTINCT ON (c.id) c.id, c.name, c.type, c.created_at,
                 m.body as last_message, m.created_at as last_message_at,
                 (SELECT COUNT(*) FROM messaging.messages WHERE conversation_id = c.id AND sender_id != $1 AND read = false) as unread
                 FROM messaging.conversations c
                 JOIN messaging.conversation_members cm ON c.id = cm.conversation_id
                 LEFT JOIN messaging.messages m ON m.id = (
                     SELECT id FROM messaging.messages WHERE conversation_id = c.id ORDER BY created_at DESC LIMIT 1
                 )
                 WHERE cm.user_id = $1
                 ORDER BY c.id, COALESCE(m.created_at, c.created_at) DESC";

    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let conversations: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "name": String::decode(&row[1]).ok(),
            "type": String::decode(&row[2]).unwrap_or_else(|_| "direct".into()),
            "created_at": String::decode(&row[3]).unwrap_or_default(),
            "last_message": String::decode(&row[4]).ok(),
            "last_message_at": String::decode(&row[5]).ok(),
            "unread_count": i64::decode(&row[6]).unwrap_or(0)
        })
    }).collect();

    json_response(200, serde_json::json!({
        "conversations": conversations
    }))
}

fn get_conversation(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conversation_id = extract_id_from_path(path, "/messages/")?;
    let conn = get_db_connection()?;

    // Verify membership
    let member_query = "SELECT 1 FROM messaging.conversation_members WHERE conversation_id = $1 AND user_id = $2";
    let member_params = [
        ParameterValue::Str(conversation_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];
    let member_rows = conn.query(member_query, &member_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if member_rows.rows.is_empty() {
        return Err(ServiceError::Forbidden("Not a member of this conversation".into()));
    }

    // Get messages
    let query = "SELECT m.id, m.sender_id, m.body, m.attachments, m.read, m.created_at, u.name, u.avatar_url
                 FROM messaging.messages m
                 LEFT JOIN users.users u ON m.sender_id = u.id
                 WHERE m.conversation_id = $1
                 ORDER BY m.created_at ASC LIMIT 100";

    let params = [ParameterValue::Str(conversation_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let messages: Vec<Message> = rows.rows.iter().map(|row| {
        Message {
            id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
            sender_id: Uuid::parse_str(&String::decode(&row[1]).unwrap_or_default()).unwrap_or_default(),
            body: String::decode(&row[2]).unwrap_or_default(),
            attachments: serde_json::from_str(&String::decode(&row[3]).unwrap_or_else(|_| "[]".into())).unwrap_or_default(),
            read: bool::decode(&row[4]).unwrap_or(false),
            created_at: String::decode(&row[5]).unwrap_or_default(),
            sender_name: String::decode(&row[6]).ok(),
            sender_avatar: String::decode(&row[7]).ok(),
        }
    }).collect();

    // Mark messages as read
    let mark_read = "UPDATE messaging.messages SET read = true 
                     WHERE conversation_id = $1 AND sender_id != $2 AND read = false";
    let mark_params = [
        ParameterValue::Str(conversation_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];
    conn.execute(mark_read, &mark_params).ok();

    json_response(200, serde_json::json!({
        "messages": messages
    }))
}

fn send_message(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: SendMessageRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    // Get or create conversation
    let conversation_id = if let Some(conv_id) = body.conversation_id {
        // Verify membership
        let member_query = "SELECT 1 FROM messaging.conversation_members WHERE conversation_id = $1 AND user_id = $2";
        let member_params = [
            ParameterValue::Str(conv_id.to_string()),
            ParameterValue::Str(user_id.to_string()),
        ];
        let member_rows = conn.query(member_query, &member_params)
            .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

        if member_rows.rows.is_empty() {
            return Err(ServiceError::Forbidden("Not a member of this conversation".into()));
        }
        conv_id
    } else if let Some(recipient_id) = body.recipient_id {
        // Create or get direct conversation
        get_or_create_direct_conversation(&conn, &user_id, &recipient_id)?
    } else {
        return Err(ServiceError::BadRequest("Either conversation_id or recipient_id required".into()));
    };

    let message_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO messaging.messages 
                  (id, conversation_id, sender_id, body, attachments, created_at)
                  VALUES ($1, $2, $3, $4, $5, $6)";

    let params = [
        ParameterValue::Str(message_id.to_string()),
        ParameterValue::Str(conversation_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.body.clone()),
        ParameterValue::Str(serde_json::to_string(&body.attachments.unwrap_or_default()).unwrap_or_else(|_| "[]".into())),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Notify other members
    let members_query = "SELECT user_id FROM messaging.conversation_members WHERE conversation_id = $1 AND user_id != $2";
    let members_params = [
        ParameterValue::Str(conversation_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];
    let members = conn.query(members_query, &members_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    for member_row in &members.rows {
        if let Ok(member_id_str) = String::decode(&member_row[0]) {
            if let Ok(member_id) = Uuid::parse_str(&member_id_str) {
                queue_event(&conn, &member_id, "message", serde_json::json!({
                    "conversation_id": conversation_id,
                    "message_id": message_id,
                    "sender_id": user_id,
                    "body": body.body,
                    "created_at": now.to_rfc3339()
                }))?;
            }
        }
    }

    json_response(201, serde_json::json!({
        "id": message_id,
        "conversation_id": conversation_id,
        "created_at": now.to_rfc3339()
    }))
}

fn delete_message(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let message_id = extract_id_from_path(path, "/messages/")?;
    let conn = get_db_connection()?;

    // Only allow deleting own messages
    let delete = "DELETE FROM messaging.messages WHERE id = $1 AND sender_id = $2";
    let params = [
        ParameterValue::Str(message_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
    ];

    let result = conn.execute(delete, &params)
        .map_err(|e| ServiceError::Internal(format!("Delete failed: {}", e)))?;

    if result == 0 {
        return Err(ServiceError::NotFound("Message not found".into()));
    }

    json_response(200, serde_json::json!({"deleted": true}))
}

//=============================================================================
// Events
//=============================================================================

fn publish_event(req: &Request) -> Result<Response, ServiceError> {
    let body: PublishEventRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;

    queue_event(&conn, &body.user_id, &body.event_type, body.data)?;

    json_response(202, serde_json::json!({
        "queued": true
    }))
}

fn subscribe_events(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    // Get pending events for user
    let query = "SELECT id, type, data, created_at FROM messaging.events 
                 WHERE user_id = $1 AND delivered = false
                 ORDER BY created_at ASC LIMIT 10";
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let events: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "type": String::decode(&row[1]).unwrap_or_default(),
            "data": serde_json::from_str::<serde_json::Value>(
                &String::decode(&row[2]).unwrap_or_else(|_| "{}".into())
            ).unwrap_or_default(),
            "created_at": String::decode(&row[3]).unwrap_or_default()
        })
    }).collect();

    // Mark as delivered
    if !events.is_empty() {
        let event_ids: Vec<String> = rows.rows.iter()
            .filter_map(|row| String::decode(&row[0]).ok())
            .collect();
        
        for event_id in event_ids {
            let mark = "UPDATE messaging.events SET delivered = true WHERE id = $1";
            conn.execute(mark, &[ParameterValue::Str(event_id)]).ok();
        }
    }

    // Return as SSE format
    let sse_data = events.iter()
        .map(|e| format!("data: {}\n\n", e))
        .collect::<Vec<_>>()
        .join("");

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Access-Control-Allow-Origin", "*")
        .body(sse_data)
        .build())
}

//=============================================================================
// Helper Functions
//=============================================================================

fn get_or_create_direct_conversation(conn: &Connection, user1: &Uuid, user2: &Uuid) -> Result<Uuid, ServiceError> {
    // Check for existing conversation
    let query = "SELECT cm1.conversation_id
                 FROM messaging.conversation_members cm1
                 JOIN messaging.conversation_members cm2 ON cm1.conversation_id = cm2.conversation_id
                 JOIN messaging.conversations c ON c.id = cm1.conversation_id
                 WHERE cm1.user_id = $1 AND cm2.user_id = $2 AND c.type = 'direct'";
    
    let params = [
        ParameterValue::Str(user1.to_string()),
        ParameterValue::Str(user2.to_string()),
    ];

    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if !rows.rows.is_empty() {
        let conv_id = String::decode(&rows.rows[0][0]).unwrap_or_default();
        return Uuid::parse_str(&conv_id)
            .map_err(|_| ServiceError::Internal("Invalid conversation ID".into()));
    }

    // Create new conversation
    let conv_id = Uuid::new_v4();
    let now = Utc::now();

    let insert_conv = "INSERT INTO messaging.conversations (id, type, created_at) VALUES ($1, 'direct', $2)";
    conn.execute(insert_conv, &[
        ParameterValue::Str(conv_id.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ]).map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    // Add members
    let insert_member = "INSERT INTO messaging.conversation_members (conversation_id, user_id, joined_at) VALUES ($1, $2, $3)";
    
    conn.execute(insert_member, &[
        ParameterValue::Str(conv_id.to_string()),
        ParameterValue::Str(user1.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ]).map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    conn.execute(insert_member, &[
        ParameterValue::Str(conv_id.to_string()),
        ParameterValue::Str(user2.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ]).map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    Ok(conv_id)
}

fn queue_event(conn: &Connection, user_id: &Uuid, event_type: &str, data: serde_json::Value) -> Result<(), ServiceError> {
    let event_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO messaging.events (id, user_id, type, data, created_at)
                  VALUES ($1, $2, $3, $4, $5)";

    let params = [
        ParameterValue::Str(event_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(event_type.to_string()),
        ParameterValue::Str(data.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Event queue failed: {}", e)))?;

    Ok(())
}

fn extract_id_from_path(path: &str, prefix: &str) -> Result<Uuid, ServiceError> {
    let id_str = path.strip_prefix(prefix)
        .and_then(|s| s.split('/').next())
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    Uuid::parse_str(id_str)
        .map_err(|_| ServiceError::BadRequest("Invalid UUID".into()))
}

fn extract_id_from_path_with_suffix(path: &str, prefix: &str, suffix: &str) -> Result<Uuid, ServiceError> {
    let id_str = path.strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
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
