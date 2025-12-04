//! AuthorWorks Content Worker
//! 
//! Background worker that processes AI content generation jobs from RabbitMQ.
//! Jobs include:
//! - Book generation (chapters, scenes)
//! - Content enhancement
//! - Summary generation
//! - Style transfer

use anyhow::Result;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum JobMessage {
    GenerateChapter {
        book_id: Uuid,
        chapter_number: u32,
        outline: String,
        context: String,
    },
    GenerateScene {
        book_id: Uuid,
        chapter_id: Uuid,
        scene_number: u32,
        outline: String,
    },
    EnhanceContent {
        content_id: Uuid,
        content: String,
        style: String,
    },
    GenerateSummary {
        book_id: Uuid,
        chapter_id: Option<Uuid>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct JobResult {
    job_id: Uuid,
    status: String,
    result: Option<String>,
    error: Option<String>,
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

    info!("Starting AuthorWorks Content Worker");

    // Get configuration from environment
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".into());
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".into());

    // Connect to PostgreSQL
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("Connected to PostgreSQL");

    // Connect to Redis
    let redis_client = redis::Client::open(redis_url)?;
    let _redis_conn = redis_client.get_multiplexed_tokio_connection().await?;
    info!("Connected to Redis");

    // Connect to RabbitMQ
    let conn = Connection::connect(&rabbitmq_url, ConnectionProperties::default()).await?;
    info!("Connected to RabbitMQ");

    let channel = conn.create_channel().await?;

    // Declare queue
    channel
        .queue_declare(
            "content_generation",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("Waiting for messages on 'content_generation' queue...");

    // Start consuming messages
    let consumer = channel
        .basic_consume(
            "content_generation",
            "content-worker",
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
                
                match serde_json::from_slice::<JobMessage>(data) {
                    Ok(job) => {
                        info!("Received job: {:?}", job);
                        
                        // Process the job
                        let result = process_job(&job, &db_pool).await;
                        
                        match result {
                            Ok(_) => {
                                info!("Job completed successfully");
                                delivery.ack(BasicAckOptions::default()).await?;
                            }
                            Err(e) => {
                                error!("Job failed: {}", e);
                                // Requeue on failure
                                delivery.nack(BasicNackOptions { requeue: true, ..Default::default() }).await?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse job message: {}", e);
                        // Don't requeue invalid messages
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

async fn process_job(job: &JobMessage, _db_pool: &sqlx::PgPool) -> Result<()> {
    match job {
        JobMessage::GenerateChapter { book_id, chapter_number, outline, context } => {
            info!("Generating chapter {} for book {}", chapter_number, book_id);
            // TODO: Call Anthropic API for generation
            // TODO: Store result in database
            warn!("Chapter generation not yet implemented");
            Ok(())
        }
        JobMessage::GenerateScene { book_id, chapter_id, scene_number, outline } => {
            info!("Generating scene {} for chapter {} in book {}", scene_number, chapter_id, book_id);
            // TODO: Call Anthropic API for generation
            warn!("Scene generation not yet implemented");
            Ok(())
        }
        JobMessage::EnhanceContent { content_id, content, style } => {
            info!("Enhancing content {} with style {}", content_id, style);
            // TODO: Call Anthropic API for enhancement
            warn!("Content enhancement not yet implemented");
            Ok(())
        }
        JobMessage::GenerateSummary { book_id, chapter_id } => {
            info!("Generating summary for book {} (chapter: {:?})", book_id, chapter_id);
            // TODO: Call Anthropic API for summarization
            warn!("Summary generation not yet implemented");
            Ok(())
        }
    }
}

