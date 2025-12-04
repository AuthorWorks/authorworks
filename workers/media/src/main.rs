//! AuthorWorks Media Worker
//! 
//! Background worker that processes media jobs from RabbitMQ.
//! Jobs include:
//! - Image resizing and optimization
//! - Audio generation (TTS)
//! - Video generation
//! - Cover image generation

use anyhow::Result;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum MediaJob {
    ResizeImage {
        source_key: String,
        target_key: String,
        width: u32,
        height: u32,
        format: String,
    },
    GenerateCover {
        book_id: Uuid,
        title: String,
        author: String,
        genre: String,
    },
    GenerateAudio {
        book_id: Uuid,
        chapter_id: Uuid,
        text: String,
        voice: String,
    },
    GenerateVideo {
        book_id: Uuid,
        chapter_id: Uuid,
        style: String,
    },
    OptimizeImage {
        source_key: String,
        target_key: String,
        quality: u8,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting AuthorWorks Media Worker");

    // Get configuration from environment
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".into());
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Connect to PostgreSQL
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("Connected to PostgreSQL");

    // Connect to RabbitMQ
    let conn = Connection::connect(&rabbitmq_url, ConnectionProperties::default()).await?;
    info!("Connected to RabbitMQ");

    let channel = conn.create_channel().await?;

    // Declare queue
    channel
        .queue_declare(
            "media_processing",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("Waiting for messages on 'media_processing' queue...");

    // Start consuming messages
    let consumer = channel
        .basic_consume(
            "media_processing",
            "media-worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    use futures_lite::StreamExt;
    let mut consumer = consumer;

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let data = &delivery.data;
                
                match serde_json::from_slice::<MediaJob>(data) {
                    Ok(job) => {
                        info!("Received media job: {:?}", job);
                        
                        let result = process_media_job(&job, &db_pool).await;
                        
                        match result {
                            Ok(_) => {
                                info!("Media job completed successfully");
                                delivery.ack(BasicAckOptions::default()).await?;
                            }
                            Err(e) => {
                                error!("Media job failed: {}", e);
                                delivery.nack(BasicNackOptions { requeue: true, ..Default::default() }).await?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse media job message: {}", e);
                        delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await?;
                    }
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
            }
        }
    }

    Ok(())
}

async fn process_media_job(job: &MediaJob, _db_pool: &sqlx::PgPool) -> Result<()> {
    match job {
        MediaJob::ResizeImage { source_key, target_key, width, height, format } => {
            info!("Resizing image {} to {}x{} as {}", source_key, width, height, format);
            
            // Use ImageMagick for resizing
            let output = Command::new("convert")
                .args([
                    "-resize", &format!("{}x{}", width, height),
                    "-format", format,
                    source_key,
                    target_key,
                ])
                .output()?;
            
            if !output.status.success() {
                anyhow::bail!("ImageMagick failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            Ok(())
        }
        MediaJob::GenerateCover { book_id, title, author, genre } => {
            info!("Generating cover for book {}: {} by {}", book_id, title, author);
            // TODO: Call image generation API (DALL-E, Stable Diffusion, etc.)
            warn!("Cover generation not yet implemented");
            Ok(())
        }
        MediaJob::GenerateAudio { book_id, chapter_id, text, voice } => {
            info!("Generating audio for chapter {} of book {} with voice {}", chapter_id, book_id, voice);
            // TODO: Call TTS API (ElevenLabs, OpenAI TTS, etc.)
            warn!("Audio generation not yet implemented");
            Ok(())
        }
        MediaJob::GenerateVideo { book_id, chapter_id, style } => {
            info!("Generating video for chapter {} of book {} in style {}", chapter_id, book_id, style);
            // TODO: Call video generation API
            warn!("Video generation not yet implemented");
            Ok(())
        }
        MediaJob::OptimizeImage { source_key, target_key, quality } => {
            info!("Optimizing image {} with quality {}", source_key, quality);
            
            let output = Command::new("convert")
                .args([
                    "-quality", &quality.to_string(),
                    "-strip",
                    source_key,
                    target_key,
                ])
                .output()?;
            
            if !output.status.success() {
                anyhow::bail!("ImageMagick optimization failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            Ok(())
        }
    }
}

