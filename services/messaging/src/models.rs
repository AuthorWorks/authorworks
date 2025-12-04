//! Data models for the Messaging Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

//=============================================================================
// Notification Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    pub read: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

//=============================================================================
// Message Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub read: bool,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub url: String,
    pub size: i64,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    pub body: String,
    pub attachments: Option<Vec<Attachment>>,
}

//=============================================================================
// Conversation Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub conversation_type: ConversationType,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationType {
    Direct,
    Group,
}

//=============================================================================
// Event Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishEventRequest {
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

//=============================================================================
// Notification Types
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    BookPublished,
    ChapterComplete,
    CommentAdded,
    MentionedInComment,
    CollaboratorAdded,
    SubscriptionExpiring,
    PaymentFailed,
    SystemAnnouncement,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::BookPublished => write!(f, "book_published"),
            NotificationType::ChapterComplete => write!(f, "chapter_complete"),
            NotificationType::CommentAdded => write!(f, "comment_added"),
            NotificationType::MentionedInComment => write!(f, "mentioned_in_comment"),
            NotificationType::CollaboratorAdded => write!(f, "collaborator_added"),
            NotificationType::SubscriptionExpiring => write!(f, "subscription_expiring"),
            NotificationType::PaymentFailed => write!(f, "payment_failed"),
            NotificationType::SystemAnnouncement => write!(f, "system_announcement"),
        }
    }
}

