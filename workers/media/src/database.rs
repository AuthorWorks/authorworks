//! Database operations for the Media Worker

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::{FileRecord, MediaJob};

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

    pub async fn get_next_media_job(&self) -> Result<Option<MediaJob>> {
        let row = sqlx::query(
            r#"
            UPDATE media.jobs
            SET status = 'processing', started_at = NOW()
            WHERE id = (
                SELECT id FROM media.jobs
                WHERE status = 'pending'
                ORDER BY created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, user_id, job_type, input
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get("id");
                let user_id: String = r.get("user_id");
                let job_type: String = r.get("job_type");
                let input: String = r.get("input");

                Ok(Some(MediaJob {
                    id: Uuid::parse_str(&id)?,
                    user_id: Uuid::parse_str(&user_id)?,
                    job_type,
                    input: serde_json::from_str(&input)?,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn update_job_status(&self, job_id: &Uuid, status: &str, error: Option<&str>, progress: Option<i32>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media.jobs
            SET status = $2, error = $3, progress = COALESCE($4, progress), updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(job_id.to_string())
        .bind(status)
        .bind(error)
        .bind(progress)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn complete_job(&self, job_id: &Uuid, output: serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media.jobs
            SET status = 'completed', output = $2, progress = 100, completed_at = NOW()
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
            UPDATE media.jobs
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

    pub async fn get_file(&self, file_id: &Uuid) -> Result<Option<FileRecord>> {
        let row = sqlx::query(
            r#"
            SELECT id, filename, s3_key, content_type
            FROM storage.files WHERE id = $1
            "#
        )
        .bind(file_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get("id");
                Ok(Some(FileRecord {
                    id: Uuid::parse_str(&id)?,
                    filename: r.get("filename"),
                    s3_key: r.get("s3_key"),
                    content_type: r.get("content_type"),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn create_file(
        &self,
        user_id: &Uuid,
        filename: &str,
        s3_key: &str,
        content_type: &str,
        size: i64,
        file_type: &str,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO storage.files (id, user_id, filename, s3_key, content_type, size, file_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            "#
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(filename)
        .bind(s3_key)
        .bind(content_type)
        .bind(size)
        .bind(file_type)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn link_thumbnail(&self, source_file_id: &Uuid, thumbnail_file_id: &Uuid, s3_key: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO media.thumbnails (id, file_id, thumbnail_file_id, s3_key, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (file_id) DO UPDATE SET thumbnail_file_id = $3, s3_key = $4
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(source_file_id.to_string())
        .bind(thumbnail_file_id.to_string())
        .bind(s3_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

