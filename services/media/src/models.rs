//! Data models for the Media Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

//=============================================================================
// Job Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: String,
    pub status: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: Uuid,
    pub job_type: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

//=============================================================================
// Request Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct ImageJobRequest {
    pub source_file_id: Uuid,
    pub operation: String,
    #[serde(default)]
    pub options: ImageOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,  // cover, contain, fill, inside, outside
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_height: Option<i32>,
    // For cover generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AudioJobRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_id: Option<Uuid>,
    pub operation: String,
    #[serde(default)]
    pub options: AudioOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,  // For TTS
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<f64>,
    // For TTS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct VideoJobRequest {
    pub source_file_id: Uuid,
    pub operation: String,
    #[serde(default)]
    pub options: VideoOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_time: Option<f64>,
}

//=============================================================================
// Job Status
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Processing => write!(f, "processing"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

