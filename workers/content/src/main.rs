//! AuthorWorks Content Worker
//!
//! Background worker for AI content generation using Anthropic Claude API.
//! Processes jobs from the queue and generates book outlines, chapters, and content enhancements.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

mod anthropic;
mod database;
mod prompts;

use anthropic::AnthropicClient;
use database::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("content_worker=info".parse()?)
        )
        .init();

    info!("Starting AuthorWorks Content Worker");

    // Load configuration
    let config = Config::from_env()?;
    
    // Initialize clients
    let db = Database::new(&config.database_url).await?;
    let anthropic = AnthropicClient::new(&config.anthropic_api_key);

    info!("Connected to database and Anthropic API");

    // Main processing loop
    loop {
        match process_next_job(&db, &anthropic).await {
            Ok(true) => {
                // Job processed, continue immediately
                continue;
            }
            Ok(false) => {
                // No jobs, wait before checking again
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("Error processing job: {}", e);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

#[derive(Debug)]
struct Config {
    database_url: String,
    anthropic_api_key: String,
    rabbitmq_url: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL not set")?,
            anthropic_api_key: env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY not set")?,
            rabbitmq_url: env::var("RABBITMQ_URL").ok(),
        })
    }
}

async fn process_next_job(db: &Database, anthropic: &AnthropicClient) -> Result<bool> {
    // Get next pending job
    let job = match db.get_next_content_job().await? {
        Some(j) => j,
        None => return Ok(false),
    };

    info!("Processing job: {} (type: {})", job.id, job.job_type);

    // Mark as processing
    db.update_job_status(&job.id, "processing", None).await?;

    // Process based on job type
    let result = match job.job_type.as_str() {
        "outline" => generate_outline(db, anthropic, &job).await,
        "chapter" => generate_chapter(db, anthropic, &job).await,
        "enhance" => enhance_content(db, anthropic, &job).await,
        other => {
            warn!("Unknown job type: {}", other);
            Err(anyhow::anyhow!("Unknown job type: {}", other))
        }
    };

    match result {
        Ok(output) => {
            info!("Job {} completed successfully", job.id);
            db.complete_job(&job.id, output).await?;
        }
        Err(e) => {
            error!("Job {} failed: {}", job.id, e);
            db.fail_job(&job.id, &e.to_string()).await?;
        }
    }

    Ok(true)
}

//=============================================================================
// Outline Generation
//=============================================================================

async fn generate_outline(db: &Database, anthropic: &AnthropicClient, job: &ContentJob) -> Result<serde_json::Value> {
    let input: OutlineInput = serde_json::from_value(job.input.clone())?;
    
    // Get book details
    let book = db.get_book(&input.book_id).await?
        .ok_or_else(|| anyhow::anyhow!("Book not found"))?;

    // Build prompt
    let prompt = prompts::build_outline_prompt(
        &book.title,
        book.description.as_deref().unwrap_or(""),
        input.genre.as_deref().unwrap_or(&book.genre.unwrap_or_default()),
        input.style.as_deref().unwrap_or("engaging and modern"),
        input.chapter_count.unwrap_or(10),
        &input.prompt,
    );

    // Call Claude API
    let response = anthropic.create_message(
        "claude-sonnet-4-20250514",
        8000,
        &prompt,
        Some("You are a professional author and book outliner. Create detailed, compelling book outlines.")
    ).await?;

    // Parse response into structured outline
    let outline = parse_outline_response(&response)?;

    // Store chapters in database
    for (i, chapter) in outline.chapters.iter().enumerate() {
        db.create_chapter(
            &input.book_id,
            &chapter.title,
            (i + 1) as i32,
            Some(&chapter.outline),
        ).await?;
    }

    // Update book metadata
    db.update_book_metadata(&input.book_id, serde_json::json!({
        "outline_generated": true,
        "synopsis": outline.synopsis,
        "themes": outline.themes
    })).await?;

    Ok(serde_json::to_value(&outline)?)
}

#[derive(Debug, Deserialize)]
struct OutlineInput {
    book_id: Uuid,
    prompt: String,
    genre: Option<String>,
    style: Option<String>,
    chapter_count: Option<i32>,
}

#[derive(Debug, Serialize)]
struct BookOutline {
    synopsis: String,
    themes: Vec<String>,
    chapters: Vec<ChapterOutline>,
}

#[derive(Debug, Serialize)]
struct ChapterOutline {
    title: String,
    outline: String,
    key_events: Vec<String>,
}

fn parse_outline_response(response: &str) -> Result<BookOutline> {
    // Try to parse as JSON first
    if let Ok(outline) = serde_json::from_str::<BookOutline>(response) {
        return Ok(outline);
    }

    // Fall back to parsing structured text
    let mut synopsis = String::new();
    let mut themes = Vec::new();
    let mut chapters = Vec::new();
    let mut current_chapter: Option<ChapterOutline> = None;

    for line in response.lines() {
        let line = line.trim();
        
        if line.starts_with("Synopsis:") || line.starts_with("## Synopsis") {
            synopsis = line.trim_start_matches("Synopsis:").trim_start_matches("## Synopsis").trim().to_string();
        } else if line.starts_with("Themes:") || line.starts_with("## Themes") {
            // Next lines are themes
        } else if line.starts_with("- ") && themes.len() < 10 && chapters.is_empty() {
            themes.push(line.trim_start_matches("- ").to_string());
        } else if line.starts_with("Chapter ") || line.starts_with("## Chapter") {
            if let Some(ch) = current_chapter.take() {
                chapters.push(ch);
            }
            let title = line
                .trim_start_matches("Chapter ")
                .trim_start_matches("## Chapter ")
                .trim_start_matches(|c: char| c.is_numeric() || c == ':' || c == '.' || c == ' ')
                .to_string();
            current_chapter = Some(ChapterOutline {
                title,
                outline: String::new(),
                key_events: Vec::new(),
            });
        } else if let Some(ref mut ch) = current_chapter {
            if line.starts_with("- ") || line.starts_with("* ") {
                ch.key_events.push(line.trim_start_matches("- ").trim_start_matches("* ").to_string());
            } else if !line.is_empty() {
                if !ch.outline.is_empty() {
                    ch.outline.push(' ');
                }
                ch.outline.push_str(line);
            }
        } else if !synopsis.is_empty() && !line.is_empty() && chapters.is_empty() {
            synopsis.push(' ');
            synopsis.push_str(line);
        }
    }

    if let Some(ch) = current_chapter {
        chapters.push(ch);
    }

    if chapters.is_empty() {
        return Err(anyhow::anyhow!("Failed to parse outline - no chapters found"));
    }

    Ok(BookOutline {
        synopsis,
        themes,
        chapters,
    })
}

//=============================================================================
// Chapter Generation
//=============================================================================

async fn generate_chapter(db: &Database, anthropic: &AnthropicClient, job: &ContentJob) -> Result<serde_json::Value> {
    let input: ChapterInput = serde_json::from_value(job.input.clone())?;
    
    // Get chapter and book details
    let chapter = db.get_chapter(&input.chapter_id).await?
        .ok_or_else(|| anyhow::anyhow!("Chapter not found"))?;
    let book = db.get_book(&chapter.book_id).await?
        .ok_or_else(|| anyhow::anyhow!("Book not found"))?;

    // Get previous chapters for context
    let previous_chapters = db.get_previous_chapters(&chapter.book_id, chapter.chapter_number).await?;
    let context = build_chapter_context(&previous_chapters);

    // Build prompt
    let prompt = prompts::build_chapter_prompt(
        &book.title,
        &chapter.title,
        chapter.chapter_number,
        input.outline.as_deref().unwrap_or(""),
        &context,
        input.style.as_deref().unwrap_or("engaging, descriptive"),
    );

    // Call Claude API with higher token limit for full chapters
    let response = anthropic.create_message(
        "claude-sonnet-4-20250514",
        16000,
        &prompt,
        Some("You are a skilled fiction writer. Write engaging, immersive prose that brings stories to life.")
    ).await?;

    // Calculate word count
    let word_count = response.split_whitespace().count() as i32;

    // Update chapter with generated content
    db.update_chapter_content(&chapter.id, &response, word_count).await?;

    Ok(serde_json::json!({
        "chapter_id": chapter.id,
        "word_count": word_count,
        "generated_at": Utc::now().to_rfc3339()
    }))
}

#[derive(Debug, Deserialize)]
struct ChapterInput {
    chapter_id: Uuid,
    outline: Option<String>,
    context: Option<String>,
    style: Option<String>,
}

fn build_chapter_context(chapters: &[ChapterSummary]) -> String {
    if chapters.is_empty() {
        return String::new();
    }

    let mut context = String::from("Previous chapters summary:\n\n");
    for ch in chapters.iter().take(3) {
        context.push_str(&format!("Chapter {}: {}\n", ch.chapter_number, ch.title));
        if let Some(ref content) = ch.content_preview {
            context.push_str(&format!("Preview: {}...\n\n", content));
        }
    }
    context
}

//=============================================================================
// Content Enhancement
//=============================================================================

async fn enhance_content(db: &Database, anthropic: &AnthropicClient, job: &ContentJob) -> Result<serde_json::Value> {
    let input: EnhanceInput = serde_json::from_value(job.input.clone())?;

    let prompt = prompts::build_enhancement_prompt(
        &input.content,
        &input.enhancement_type,
        input.instructions.as_deref(),
    );

    let response = anthropic.create_message(
        "claude-sonnet-4-20250514",
        8000,
        &prompt,
        Some("You are an expert editor. Improve the given content while maintaining the author's voice.")
    ).await?;

    // If chapter_id is provided, update the chapter
    if let Some(chapter_id) = input.chapter_id {
        let word_count = response.split_whitespace().count() as i32;
        db.update_chapter_content(&chapter_id, &response, word_count).await?;
    }

    Ok(serde_json::json!({
        "enhanced_content": response,
        "enhancement_type": input.enhancement_type,
        "original_word_count": input.content.split_whitespace().count(),
        "enhanced_word_count": response.split_whitespace().count()
    }))
}

#[derive(Debug, Deserialize)]
struct EnhanceInput {
    chapter_id: Option<Uuid>,
    content: String,
    enhancement_type: String,
    instructions: Option<String>,
}

//=============================================================================
// Data Models
//=============================================================================

#[derive(Debug)]
pub struct ContentJob {
    pub id: Uuid,
    pub book_id: Uuid,
    pub job_type: String,
    pub input: serde_json::Value,
}

#[derive(Debug)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub genre: Option<String>,
}

#[derive(Debug)]
pub struct Chapter {
    pub id: Uuid,
    pub book_id: Uuid,
    pub title: String,
    pub chapter_number: i32,
    pub content: Option<String>,
}

#[derive(Debug)]
pub struct ChapterSummary {
    pub chapter_number: i32,
    pub title: String,
    pub content_preview: Option<String>,
}
