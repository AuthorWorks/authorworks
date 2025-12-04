//! Database operations for the Content Worker

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::{Book, Chapter, ChapterSummary, ContentJob};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .context("Failed to connect to database")?;

        Ok(Self { pool })
    }

    pub async fn get_next_content_job(&self) -> Result<Option<ContentJob>> {
        let row = sqlx::query(
            r#"
            UPDATE content.generation_jobs
            SET status = 'processing', started_at = NOW()
            WHERE id = (
                SELECT id FROM content.generation_jobs
                WHERE status = 'pending'
                ORDER BY created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, book_id, job_type, input
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get("id");
                let book_id: String = r.get("book_id");
                let job_type: String = r.get("job_type");
                let input: String = r.get("input");

                Ok(Some(ContentJob {
                    id: Uuid::parse_str(&id)?,
                    book_id: Uuid::parse_str(&book_id)?,
                    job_type,
                    input: serde_json::from_str(&input)?,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn update_job_status(&self, job_id: &Uuid, status: &str, error: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE content.generation_jobs
            SET status = $2, error = $3, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(job_id.to_string())
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn complete_job(&self, job_id: &Uuid, output: serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE content.generation_jobs
            SET status = 'completed', output = $2, completed_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(job_id.to_string())
        .bind(output.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fail_job(&self, job_id: &Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE content.generation_jobs
            SET status = 'failed', error = $2, completed_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(job_id.to_string())
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_book(&self, book_id: &Uuid) -> Result<Option<Book>> {
        let row = sqlx::query(
            r#"
            SELECT id, title, description, genre
            FROM content.books WHERE id = $1
            "#
        )
        .bind(book_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get("id");
                Ok(Some(Book {
                    id: Uuid::parse_str(&id)?,
                    title: r.get("title"),
                    description: r.try_get("description").ok(),
                    genre: r.try_get("genre").ok(),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_chapter(&self, chapter_id: &Uuid) -> Result<Option<Chapter>> {
        let row = sqlx::query(
            r#"
            SELECT id, book_id, title, chapter_number, content
            FROM content.chapters WHERE id = $1
            "#
        )
        .bind(chapter_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get("id");
                let book_id: String = r.get("book_id");
                Ok(Some(Chapter {
                    id: Uuid::parse_str(&id)?,
                    book_id: Uuid::parse_str(&book_id)?,
                    title: r.get("title"),
                    chapter_number: r.get("chapter_number"),
                    content: r.try_get("content").ok(),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_previous_chapters(&self, book_id: &Uuid, before_number: i32) -> Result<Vec<ChapterSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT chapter_number, title, LEFT(content, 500) as content_preview
            FROM content.chapters
            WHERE book_id = $1 AND chapter_number < $2
            ORDER BY chapter_number DESC
            LIMIT 3
            "#
        )
        .bind(book_id.to_string())
        .bind(before_number)
        .fetch_all(&self.pool)
        .await?;

        let chapters = rows.iter().map(|r| {
            ChapterSummary {
                chapter_number: r.get("chapter_number"),
                title: r.get("title"),
                content_preview: r.try_get("content_preview").ok(),
            }
        }).collect();

        Ok(chapters)
    }

    pub async fn create_chapter(&self, book_id: &Uuid, title: &str, chapter_number: i32, outline: Option<&str>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        
        sqlx::query(
            r#"
            INSERT INTO content.chapters (id, book_id, title, chapter_number, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'draft', NOW(), NOW())
            "#
        )
        .bind(id.to_string())
        .bind(book_id.to_string())
        .bind(title)
        .bind(chapter_number)
        .execute(&self.pool)
        .await?;

        // Store outline in metadata if provided
        if let Some(outline) = outline {
            sqlx::query(
                r#"
                UPDATE content.chapters
                SET metadata = jsonb_set(COALESCE(metadata, '{}'), '{outline}', $2::jsonb)
                WHERE id = $1
                "#
            )
            .bind(id.to_string())
            .bind(serde_json::json!(outline).to_string())
            .execute(&self.pool)
            .await?;
        }

        Ok(id)
    }

    pub async fn update_chapter_content(&self, chapter_id: &Uuid, content: &str, word_count: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE content.chapters
            SET content = $2, word_count = $3, status = 'draft', updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(chapter_id.to_string())
        .bind(content)
        .bind(word_count)
        .execute(&self.pool)
        .await?;

        // Update book word count
        sqlx::query(
            r#"
            UPDATE content.books
            SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM content.chapters WHERE book_id = (SELECT book_id FROM content.chapters WHERE id = $1)),
                updated_at = NOW()
            WHERE id = (SELECT book_id FROM content.chapters WHERE id = $1)
            "#
        )
        .bind(chapter_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_book_metadata(&self, book_id: &Uuid, metadata: serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE content.books
            SET metadata = COALESCE(metadata, '{}') || $2::jsonb, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(book_id.to_string())
        .bind(metadata.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn track_ai_usage(&self, user_id: &Uuid, word_count: i32) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO subscriptions.ai_usage (id, user_id, word_count, created_at)
            VALUES ($1, $2, $3, NOW())
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id.to_string())
        .bind(word_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

