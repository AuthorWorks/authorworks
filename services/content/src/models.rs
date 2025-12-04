//! Data models for the Content Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

//=============================================================================
// Book Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image_url: Option<String>,
    pub word_count: i32,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSummary {
    pub id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image_url: Option<String>,
    pub word_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBookRequest {
    pub title: String,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBookRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub status: Option<String>,
    pub cover_image_url: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

//=============================================================================
// Chapter Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Uuid,
    pub book_id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub chapter_number: i32,
    pub word_count: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub id: Uuid,
    pub title: String,
    pub chapter_number: i32,
    pub word_count: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateChapterRequest {
    pub title: String,
    pub chapter_number: i32,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChapterRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
}

//=============================================================================
// Scene Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub title: String,
    pub content: Option<String>,
    pub scene_number: i32,
    pub word_count: i32,
    pub pov_character: Option<String>,
    pub location: Option<String>,
    pub time_period: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

//=============================================================================
// Generation Request Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct GenerateOutlineRequest {
    pub book_id: Uuid,
    pub prompt: String,
    pub genre: Option<String>,
    pub style: Option<String>,
    pub chapter_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateChapterRequest {
    pub chapter_id: Uuid,
    pub outline: Option<String>,
    pub context: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnhanceContentRequest {
    pub chapter_id: Uuid,
    pub content: String,
    pub enhancement_type: EnhancementType,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementType {
    Grammar,
    Style,
    Dialog,
    Description,
    Pacing,
    Continuity,
    All,
}

//=============================================================================
// Generation Job Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationJob {
    pub id: Uuid,
    pub book_id: Uuid,
    pub job_type: String,
    pub status: JobStatus,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

//=============================================================================
// Character Models (for reference in content)
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterReference {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub description: Option<String>,
}

//=============================================================================
// Book Status Enum
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BookStatus {
    Draft,
    Writing,
    Editing,
    Review,
    Published,
    Archived,
}

impl Default for BookStatus {
    fn default() -> Self {
        BookStatus::Draft
    }
}

impl std::fmt::Display for BookStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookStatus::Draft => write!(f, "draft"),
            BookStatus::Writing => write!(f, "writing"),
            BookStatus::Editing => write!(f, "editing"),
            BookStatus::Review => write!(f, "review"),
            BookStatus::Published => write!(f, "published"),
            BookStatus::Archived => write!(f, "archived"),
        }
    }
}

