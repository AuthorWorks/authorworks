//! Data models for the Storage Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

//=============================================================================
// S3 Configuration
//=============================================================================

#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

//=============================================================================
// File Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: Uuid,
    pub filename: String,
    pub s3_key: String,
    pub content_type: String,
    pub size: i64,
    pub checksum: String,
    pub file_type: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub file_type: String,
    pub created_at: String,
}

//=============================================================================
// Request Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct DirectUploadRequest {
    pub filename: String,
    pub content: String,  // Base64 encoded
    pub content_type: String,
    pub file_type: String,  // cover, manuscript, audio, etc.
    pub size: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PresignedUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub file_type: String,
    pub size: i64,
}

#[derive(Debug, Deserialize)]
pub struct CopyFileRequest {
    pub new_filename: Option<String>,
}

//=============================================================================
// File Types
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Cover,
    Manuscript,
    Audio,
    Video,
    Image,
    Document,
    Other,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Cover => write!(f, "cover"),
            FileType::Manuscript => write!(f, "manuscript"),
            FileType::Audio => write!(f, "audio"),
            FileType::Video => write!(f, "video"),
            FileType::Image => write!(f, "image"),
            FileType::Document => write!(f, "document"),
            FileType::Other => write!(f, "other"),
        }
    }
}

