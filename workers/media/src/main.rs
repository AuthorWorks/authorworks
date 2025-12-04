//! AuthorWorks Media Worker
//!
//! Background worker for media processing using FFmpeg, ImageMagick, and AI APIs.
//! Handles image transformations, audio processing, video encoding, and AI media generation.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tokio::fs;
use tracing::{error, info, warn};
use uuid::Uuid;

mod database;
mod s3;

use database::Database;
use s3::S3Client;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("media_worker=info".parse()?)
        )
        .init();

    info!("Starting AuthorWorks Media Worker");

    let config = Config::from_env()?;
    let db = Database::new(&config.database_url).await?;
    let s3 = S3Client::new(
        &config.s3_endpoint,
        &config.s3_access_key,
        &config.s3_secret_key,
        &config.s3_bucket,
    );

    // Create temp directory
    fs::create_dir_all(&config.temp_dir).await?;

    info!("Connected to database and S3");

    loop {
        match process_next_job(&db, &s3, &config).await {
            Ok(true) => continue,
            Ok(false) => sleep(Duration::from_secs(5)).await,
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
    s3_endpoint: String,
    s3_access_key: String,
    s3_secret_key: String,
    s3_bucket: String,
    temp_dir: PathBuf,
    openai_api_key: Option<String>,
    elevenlabs_api_key: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://minio:9000".into()),
            s3_access_key: env::var("S3_ACCESS_KEY").context("S3_ACCESS_KEY not set")?,
            s3_secret_key: env::var("S3_SECRET_KEY").context("S3_SECRET_KEY not set")?,
            s3_bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "authorworks".into()),
            temp_dir: PathBuf::from(env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp/media-worker".into())),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            elevenlabs_api_key: env::var("ELEVENLABS_API_KEY").ok(),
        })
    }
}

async fn process_next_job(db: &Database, s3: &S3Client, config: &Config) -> Result<bool> {
    let job = match db.get_next_media_job().await? {
        Some(j) => j,
        None => return Ok(false),
    };

    info!("Processing job: {} (type: {})", job.id, job.job_type);
    db.update_job_status(&job.id, "processing", None, None).await?;

    let result = match job.job_type.as_str() {
        "image" => process_image_job(db, s3, config, &job).await,
        "audio" => process_audio_job(db, s3, config, &job).await,
        "video" => process_video_job(db, s3, config, &job).await,
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

    // Cleanup temp files
    let job_temp_dir = config.temp_dir.join(job.id.to_string());
    if job_temp_dir.exists() {
        fs::remove_dir_all(&job_temp_dir).await.ok();
    }

    Ok(true)
}

//=============================================================================
// Image Processing
//=============================================================================

async fn process_image_job(db: &Database, s3: &S3Client, config: &Config, job: &MediaJob) -> Result<serde_json::Value> {
    let input: ImageInput = serde_json::from_value(job.input.clone())?;

    match input.operation.as_str() {
        "resize" | "crop" | "compress" | "convert" | "thumbnail" => {
            process_image_transform(db, s3, config, job, &input).await
        }
        "cover_generation" => {
            generate_cover_image(s3, config, job, &input).await
        }
        _ => Err(anyhow::anyhow!("Unknown image operation: {}", input.operation))
    }
}

async fn process_image_transform(db: &Database, s3: &S3Client, config: &Config, job: &MediaJob, input: &ImageInput) -> Result<serde_json::Value> {
    let source_file_id = input.source_file_id
        .ok_or_else(|| anyhow::anyhow!("source_file_id required"))?;

    // Get source file info
    let source = db.get_file(&source_file_id).await?
        .ok_or_else(|| anyhow::anyhow!("Source file not found"))?;

    // Download from S3
    let job_dir = config.temp_dir.join(job.id.to_string());
    fs::create_dir_all(&job_dir).await?;

    let input_path = job_dir.join(&source.filename);
    s3.download_file(&source.s3_key, &input_path).await?;

    // Determine output format
    let output_format = input.options.format.as_deref().unwrap_or("webp");
    let output_filename = format!("{}.{}", Uuid::new_v4(), output_format);
    let output_path = job_dir.join(&output_filename);

    // Build ImageMagick command
    let mut args = vec![input_path.to_string_lossy().to_string()];

    // Apply operations
    match input.operation.as_str() {
        "resize" => {
            if let (Some(w), Some(h)) = (input.options.width, input.options.height) {
                let fit = input.options.fit.as_deref().unwrap_or("inside");
                let geometry = match fit {
                    "cover" => format!("{}x{}^", w, h),
                    "contain" | "inside" => format!("{}x{}>", w, h),
                    "fill" => format!("{}x{}!", w, h),
                    _ => format!("{}x{}", w, h),
                };
                args.extend(vec!["-resize".to_string(), geometry]);
                if fit == "cover" {
                    args.extend(vec!["-gravity".to_string(), "center".to_string(), 
                                     "-extent".to_string(), format!("{}x{}", w, h)]);
                }
            }
        }
        "crop" => {
            if let (Some(x), Some(y), Some(w), Some(h)) = 
                (input.options.crop_x, input.options.crop_y, input.options.crop_width, input.options.crop_height) {
                args.extend(vec!["-crop".to_string(), format!("{}x{}+{}+{}", w, h, x, y)]);
            }
        }
        "thumbnail" => {
            let w = input.options.width.unwrap_or(300);
            let h = input.options.height.unwrap_or(400);
            args.extend(vec![
                "-thumbnail".to_string(), format!("{}x{}^", w, h),
                "-gravity".to_string(), "center".to_string(),
                "-extent".to_string(), format!("{}x{}", w, h),
            ]);
        }
        _ => {}
    }

    // Quality
    let quality = input.options.quality.unwrap_or(85);
    args.extend(vec!["-quality".to_string(), quality.to_string()]);

    // Strip metadata for smaller files
    args.push("-strip".to_string());

    // Output
    args.push(output_path.to_string_lossy().to_string());

    // Execute ImageMagick
    let output = Command::new("convert")
        .args(&args)
        .output()
        .context("Failed to execute ImageMagick")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "ImageMagick failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Upload result to S3
    let s3_key = format!("{}/processed/{}", job.user_id, output_filename);
    s3.upload_file(&output_path, &s3_key, &format!("image/{}", output_format)).await?;

    // Get file size
    let metadata = fs::metadata(&output_path).await?;

    // Store file record
    let file_id = db.create_file(
        &job.user_id,
        &output_filename,
        &s3_key,
        &format!("image/{}", output_format),
        metadata.len() as i64,
        "processed",
    ).await?;

    // For thumbnails, link to source
    if input.operation == "thumbnail" {
        db.link_thumbnail(&source_file_id, &file_id, &s3_key).await?;
    }

    Ok(serde_json::json!({
        "file_id": file_id,
        "s3_key": s3_key,
        "size": metadata.len(),
        "format": output_format
    }))
}

async fn generate_cover_image(s3: &S3Client, config: &Config, job: &MediaJob, input: &ImageInput) -> Result<serde_json::Value> {
    let openai_key = config.openai_api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY required for cover generation"))?;

    let prompt = input.options.prompt.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Prompt required for cover generation"))?;

    let style = input.options.style.as_deref().unwrap_or("digital art, book cover style");

    // Call DALL-E API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/images/generations")
        .header("Authorization", format!("Bearer {}", openai_key))
        .json(&serde_json::json!({
            "model": "dall-e-3",
            "prompt": format!("{} Style: {}", prompt, style),
            "n": 1,
            "size": "1024x1792",
            "quality": "hd"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("DALL-E API error: {}", error));
    }

    let result: serde_json::Value = response.json().await?;
    let image_url = result["data"][0]["url"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No image URL in response"))?;

    // Download generated image
    let image_response = client.get(image_url).send().await?;
    let image_bytes = image_response.bytes().await?;

    // Save and upload
    let job_dir = config.temp_dir.join(job.id.to_string());
    fs::create_dir_all(&job_dir).await?;

    let filename = format!("{}.png", Uuid::new_v4());
    let local_path = job_dir.join(&filename);
    fs::write(&local_path, &image_bytes).await?;

    let s3_key = format!("{}/covers/{}", job.user_id, filename);
    s3.upload_file(&local_path, &s3_key, "image/png").await?;

    Ok(serde_json::json!({
        "s3_key": s3_key,
        "size": image_bytes.len(),
        "prompt": prompt
    }))
}

//=============================================================================
// Audio Processing
//=============================================================================

async fn process_audio_job(db: &Database, s3: &S3Client, config: &Config, job: &MediaJob) -> Result<serde_json::Value> {
    let input: AudioInput = serde_json::from_value(job.input.clone())?;

    match input.operation.as_str() {
        "convert" | "compress" | "trim" | "normalize" => {
            process_audio_transform(db, s3, config, job, &input).await
        }
        "tts" => {
            generate_tts_audio(s3, config, job, &input).await
        }
        _ => Err(anyhow::anyhow!("Unknown audio operation: {}", input.operation))
    }
}

async fn process_audio_transform(db: &Database, s3: &S3Client, config: &Config, job: &MediaJob, input: &AudioInput) -> Result<serde_json::Value> {
    let source_file_id = input.source_file_id
        .ok_or_else(|| anyhow::anyhow!("source_file_id required"))?;

    let source = db.get_file(&source_file_id).await?
        .ok_or_else(|| anyhow::anyhow!("Source file not found"))?;

    let job_dir = config.temp_dir.join(job.id.to_string());
    fs::create_dir_all(&job_dir).await?;

    let input_path = job_dir.join(&source.filename);
    s3.download_file(&source.s3_key, &input_path).await?;

    let output_format = input.options.format.as_deref().unwrap_or("mp3");
    let output_filename = format!("{}.{}", Uuid::new_v4(), output_format);
    let output_path = job_dir.join(&output_filename);

    // Build FFmpeg command
    let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

    // Audio codec based on format
    match output_format {
        "mp3" => {
            args.extend(vec!["-codec:a".to_string(), "libmp3lame".to_string()]);
            let bitrate = input.options.bitrate.as_deref().unwrap_or("192k");
            args.extend(vec!["-b:a".to_string(), bitrate.to_string()]);
        }
        "aac" | "m4a" => {
            args.extend(vec!["-codec:a".to_string(), "aac".to_string()]);
            let bitrate = input.options.bitrate.as_deref().unwrap_or("192k");
            args.extend(vec!["-b:a".to_string(), bitrate.to_string()]);
        }
        "wav" => {
            args.extend(vec!["-codec:a".to_string(), "pcm_s16le".to_string()]);
        }
        "ogg" => {
            args.extend(vec!["-codec:a".to_string(), "libvorbis".to_string()]);
        }
        _ => {}
    }

    // Sample rate
    if let Some(sr) = input.options.sample_rate {
        args.extend(vec!["-ar".to_string(), sr.to_string()]);
    }

    // Channels
    if let Some(ch) = input.options.channels {
        args.extend(vec!["-ac".to_string(), ch.to_string()]);
    }

    // Trim
    if let Some(start) = input.options.start_time {
        args.extend(vec!["-ss".to_string(), start.to_string()]);
    }
    if let Some(end) = input.options.end_time {
        args.extend(vec!["-to".to_string(), end.to_string()]);
    }

    // Normalize
    if input.operation == "normalize" {
        args.extend(vec!["-af".to_string(), "loudnorm=I=-16:TP=-1.5:LRA=11".to_string()]);
    }

    args.extend(vec!["-y".to_string(), output_path.to_string_lossy().to_string()]);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute FFmpeg")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "FFmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let s3_key = format!("{}/audio/{}", job.user_id, output_filename);
    s3.upload_file(&output_path, &s3_key, &format!("audio/{}", output_format)).await?;

    let metadata = fs::metadata(&output_path).await?;

    let file_id = db.create_file(
        &job.user_id,
        &output_filename,
        &s3_key,
        &format!("audio/{}", output_format),
        metadata.len() as i64,
        "audio",
    ).await?;

    Ok(serde_json::json!({
        "file_id": file_id,
        "s3_key": s3_key,
        "size": metadata.len(),
        "format": output_format
    }))
}

async fn generate_tts_audio(s3: &S3Client, config: &Config, job: &MediaJob, input: &AudioInput) -> Result<serde_json::Value> {
    let text = input.text.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Text required for TTS"))?;

    // Try ElevenLabs first, fall back to OpenAI
    let audio_bytes = if let Some(api_key) = &config.elevenlabs_api_key {
        generate_elevenlabs_tts(api_key, text, &input.options).await?
    } else if let Some(api_key) = &config.openai_api_key {
        generate_openai_tts(api_key, text, &input.options).await?
    } else {
        return Err(anyhow::anyhow!("No TTS API key configured"));
    };

    let job_dir = config.temp_dir.join(job.id.to_string());
    fs::create_dir_all(&job_dir).await?;

    let filename = format!("{}.mp3", Uuid::new_v4());
    let local_path = job_dir.join(&filename);
    fs::write(&local_path, &audio_bytes).await?;

    let s3_key = format!("{}/tts/{}", job.user_id, filename);
    s3.upload_file(&local_path, &s3_key, "audio/mpeg").await?;

    Ok(serde_json::json!({
        "s3_key": s3_key,
        "size": audio_bytes.len(),
        "text_length": text.len()
    }))
}

async fn generate_elevenlabs_tts(api_key: &str, text: &str, options: &AudioOptions) -> Result<Vec<u8>> {
    let voice_id = options.voice.as_deref().unwrap_or("21m00Tcm4TlvDq8ikWAM"); // Rachel

    let client = reqwest::Client::new();
    let response = client
        .post(format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id))
        .header("xi-api-key", api_key)
        .json(&serde_json::json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.75
            }
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("ElevenLabs API error: {}", error));
    }

    Ok(response.bytes().await?.to_vec())
}

async fn generate_openai_tts(api_key: &str, text: &str, options: &AudioOptions) -> Result<Vec<u8>> {
    let voice = options.voice.as_deref().unwrap_or("alloy");
    let speed = options.speed.unwrap_or(1.0);

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "tts-1-hd",
            "input": text,
            "voice": voice,
            "speed": speed
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("OpenAI TTS error: {}", error));
    }

    Ok(response.bytes().await?.to_vec())
}

//=============================================================================
// Video Processing
//=============================================================================

async fn process_video_job(db: &Database, s3: &S3Client, config: &Config, job: &MediaJob) -> Result<serde_json::Value> {
    let input: VideoInput = serde_json::from_value(job.input.clone())?;

    let source_file_id = input.source_file_id;
    let source = db.get_file(&source_file_id).await?
        .ok_or_else(|| anyhow::anyhow!("Source file not found"))?;

    let job_dir = config.temp_dir.join(job.id.to_string());
    fs::create_dir_all(&job_dir).await?;

    let input_path = job_dir.join(&source.filename);
    s3.download_file(&source.s3_key, &input_path).await?;

    match input.operation.as_str() {
        "convert" | "compress" => {
            process_video_transcode(db, s3, job, &input_path, &job_dir, &input).await
        }
        "thumbnail" => {
            extract_video_thumbnail(db, s3, job, &input_path, &job_dir, &input).await
        }
        "trailer" => {
            create_video_trailer(db, s3, job, &input_path, &job_dir, &input).await
        }
        _ => Err(anyhow::anyhow!("Unknown video operation: {}", input.operation))
    }
}

async fn process_video_transcode(db: &Database, s3: &S3Client, job: &MediaJob, input_path: &Path, job_dir: &Path, input: &VideoInput) -> Result<serde_json::Value> {
    let output_format = input.options.format.as_deref().unwrap_or("mp4");
    let output_filename = format!("{}.{}", Uuid::new_v4(), output_format);
    let output_path = job_dir.join(&output_filename);

    let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

    // Video codec
    args.extend(vec!["-c:v".to_string(), "libx264".to_string()]);

    // Resolution
    if let Some(res) = &input.options.resolution {
        args.extend(vec!["-vf".to_string(), format!("scale={}", res.replace("x", ":"))]);
    }

    // Bitrate
    if let Some(br) = &input.options.bitrate {
        args.extend(vec!["-b:v".to_string(), br.clone()]);
    }

    // FPS
    if let Some(fps) = input.options.fps {
        args.extend(vec!["-r".to_string(), fps.to_string()]);
    }

    // Duration
    if let Some(start) = input.options.start_time {
        args.extend(vec!["-ss".to_string(), start.to_string()]);
    }
    if let Some(dur) = input.options.duration {
        args.extend(vec!["-t".to_string(), dur.to_string()]);
    }

    // Audio
    args.extend(vec!["-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string()]);

    // Fast start for web
    args.extend(vec!["-movflags".to_string(), "+faststart".to_string()]);

    args.extend(vec!["-y".to_string(), output_path.to_string_lossy().to_string()]);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute FFmpeg")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "FFmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let s3_key = format!("{}/video/{}", job.user_id, output_filename);
    s3.upload_file(&output_path, &s3_key, &format!("video/{}", output_format)).await?;

    let metadata = fs::metadata(&output_path).await?;

    let file_id = db.create_file(
        &job.user_id,
        &output_filename,
        &s3_key,
        &format!("video/{}", output_format),
        metadata.len() as i64,
        "video",
    ).await?;

    Ok(serde_json::json!({
        "file_id": file_id,
        "s3_key": s3_key,
        "size": metadata.len(),
        "format": output_format
    }))
}

async fn extract_video_thumbnail(db: &Database, s3: &S3Client, job: &MediaJob, input_path: &Path, job_dir: &Path, input: &VideoInput) -> Result<serde_json::Value> {
    let timestamp = input.options.thumbnail_time.unwrap_or(1.0);
    let filename = format!("{}.jpg", Uuid::new_v4());
    let output_path = job_dir.join(&filename);

    let args = vec![
        "-i".to_string(), input_path.to_string_lossy().to_string(),
        "-ss".to_string(), timestamp.to_string(),
        "-vframes".to_string(), "1".to_string(),
        "-vf".to_string(), "scale=640:-1".to_string(),
        "-y".to_string(), output_path.to_string_lossy().to_string(),
    ];

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute FFmpeg")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "FFmpeg thumbnail extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let s3_key = format!("{}/thumbnails/{}", job.user_id, filename);
    s3.upload_file(&output_path, &s3_key, "image/jpeg").await?;

    Ok(serde_json::json!({
        "s3_key": s3_key,
        "timestamp": timestamp
    }))
}

async fn create_video_trailer(db: &Database, s3: &S3Client, job: &MediaJob, input_path: &Path, job_dir: &Path, input: &VideoInput) -> Result<serde_json::Value> {
    let duration = input.options.duration.unwrap_or(30.0);
    let filename = format!("{}_trailer.mp4", Uuid::new_v4());
    let output_path = job_dir.join(&filename);

    // Create trailer: first N seconds with fade out
    let args = vec![
        "-i".to_string(), input_path.to_string_lossy().to_string(),
        "-t".to_string(), duration.to_string(),
        "-vf".to_string(), format!("fade=t=out:st={}:d=2", duration - 2.0),
        "-af".to_string(), format!("afade=t=out:st={}:d=2", duration - 2.0),
        "-c:v".to_string(), "libx264".to_string(),
        "-c:a".to_string(), "aac".to_string(),
        "-movflags".to_string(), "+faststart".to_string(),
        "-y".to_string(), output_path.to_string_lossy().to_string(),
    ];

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute FFmpeg")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "FFmpeg trailer creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let s3_key = format!("{}/trailers/{}", job.user_id, filename);
    s3.upload_file(&output_path, &s3_key, "video/mp4").await?;

    let metadata = fs::metadata(&output_path).await?;

    let file_id = db.create_file(
        &job.user_id,
        &filename,
        &s3_key,
        "video/mp4",
        metadata.len() as i64,
        "video",
    ).await?;

    Ok(serde_json::json!({
        "file_id": file_id,
        "s3_key": s3_key,
        "duration": duration
    }))
}

//=============================================================================
// Data Models
//=============================================================================

#[derive(Debug)]
pub struct MediaJob {
    pub id: Uuid,
    pub user_id: Uuid,
    pub job_type: String,
    pub input: serde_json::Value,
}

#[derive(Debug)]
pub struct FileRecord {
    pub id: Uuid,
    pub filename: String,
    pub s3_key: String,
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
struct ImageInput {
    operation: String,
    source_file_id: Option<Uuid>,
    #[serde(default)]
    options: ImageOptions,
}

#[derive(Debug, Default, Deserialize)]
struct ImageOptions {
    width: Option<i32>,
    height: Option<i32>,
    quality: Option<i32>,
    format: Option<String>,
    fit: Option<String>,
    crop_x: Option<i32>,
    crop_y: Option<i32>,
    crop_width: Option<i32>,
    crop_height: Option<i32>,
    prompt: Option<String>,
    style: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudioInput {
    operation: String,
    source_file_id: Option<Uuid>,
    text: Option<String>,
    #[serde(default)]
    options: AudioOptions,
}

#[derive(Debug, Default, Deserialize)]
struct AudioOptions {
    format: Option<String>,
    bitrate: Option<String>,
    sample_rate: Option<i32>,
    channels: Option<i32>,
    start_time: Option<f64>,
    end_time: Option<f64>,
    voice: Option<String>,
    speed: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct VideoInput {
    operation: String,
    source_file_id: Uuid,
    #[serde(default)]
    options: VideoOptions,
}

#[derive(Debug, Default, Deserialize)]
struct VideoOptions {
    format: Option<String>,
    resolution: Option<String>,
    bitrate: Option<String>,
    fps: Option<i32>,
    start_time: Option<f64>,
    duration: Option<f64>,
    thumbnail_time: Option<f64>,
}
