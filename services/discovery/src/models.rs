//! Data models for the Discovery Service

use serde::{Deserialize, Serialize};

//=============================================================================
// Search Result Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    #[serde(rename = "type")]
    pub result_type: String,
    pub title: String,
    pub description: Option<String>,
    pub highlight: Option<serde_json::Value>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSearchResult {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub author_name: Option<String>,
    pub genre: Option<String>,
    pub status: String,
    pub cover_url: Option<String>,
    pub word_count: i32,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSearchResult {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub chapter_number: i32,
    pub highlight: Option<Vec<String>>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorSearchResult {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub book_count: i32,
    pub score: f64,
}

//=============================================================================
// Index Request Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct IndexBookRequest {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub author_id: String,
    pub author_name: Option<String>,
    pub genre: Option<String>,
    pub status: String,
    pub cover_url: Option<String>,
    pub word_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct IndexChapterRequest {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub content: Option<String>,
    pub chapter_number: i32,
    pub word_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct IndexAuthorRequest {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub genres: Vec<String>,
    pub book_count: i32,
}

