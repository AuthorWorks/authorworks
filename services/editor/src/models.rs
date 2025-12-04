//! Data models for the Editor Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;

//=============================================================================
// Operation Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    Insert { position: i32, text: String },
    Delete { position: i32, length: i32 },
    Replace { position: i32, length: i32, text: String },
    Revert { checkpoint_id: Uuid },
}

#[derive(Debug, Deserialize)]
pub struct OperationRequest {
    pub base_version: i64,
    pub operation: Operation,
}

//=============================================================================
// Checkpoint Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct CheckpointRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct RevertRequest {
    pub checkpoint_id: Uuid,
}

//=============================================================================
// Presence Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub start: i32,
    pub end: i32,
}

#[derive(Debug, Deserialize)]
pub struct PresenceUpdate {
    pub cursor_position: Option<i32>,
    pub selection: Option<Selection>,
}

//=============================================================================
// Comment Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub start: i32,
    pub end: i32,
}

#[derive(Debug, Deserialize)]
pub struct CommentRequest {
    pub content: String,
    pub position: Position,
}

