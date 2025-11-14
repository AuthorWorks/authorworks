use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::error::Result;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tracing::info;
use chrono::Local;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Token pricing constants (per 1M tokens)
const CLAUDE_INPUT_PRICE: f64 = 3.0;  // $3 per 1M input tokens
const CLAUDE_OUTPUT_PRICE: f64 = 15.0; // $15 per 1M output tokens

#[derive(Debug, Default, Clone)]
pub struct TokenUsageStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub operations: u32,
}

impl TokenUsageStats {
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
    
    pub fn calculate_cost(&self) -> f64 {
        let prompt_cost = (self.prompt_tokens as f64 / 1_000_000.0) * CLAUDE_INPUT_PRICE;
        let completion_cost = (self.completion_tokens as f64 / 1_000_000.0) * CLAUDE_OUTPUT_PRICE;
        prompt_cost + completion_cost
    }
}

/// Token tracker for tracking token usage
#[derive(Debug, Clone)]
pub struct TokenTracker {
    pub prompt_tokens: Arc<Mutex<usize>>,
    pub completion_tokens: Arc<Mutex<usize>>,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenTracker {
    /// Create a new token tracker
    pub fn new() -> Self {
        Self {
            prompt_tokens: Arc::new(Mutex::new(0)),
            completion_tokens: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Add prompt tokens
    pub fn add_prompt_tokens(&self, tokens: usize) {
        let mut prompt_tokens = self.prompt_tokens.lock().unwrap();
        *prompt_tokens += tokens;
    }
    
    /// Add completion tokens
    pub fn add_completion_tokens(&self, tokens: usize) {
        let mut completion_tokens = self.completion_tokens.lock().unwrap();
        *completion_tokens += tokens;
    }
    
    /// Get the number of prompt tokens
    pub fn get_prompt_tokens(&self) -> usize {
        *self.prompt_tokens.lock().unwrap()
    }
    
    /// Get the number of completion tokens
    pub fn get_completion_tokens(&self) -> usize {
        *self.completion_tokens.lock().unwrap()
    }
    
    /// Get the total tokens
    pub fn get_total_tokens(&self) -> usize {
        self.get_prompt_tokens() + self.get_completion_tokens()
    }
}

pub fn log_prompt(output_dir: &Path, name: &str, prompt: &str, context: &str) -> Result<()> {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();
    
    let log_dir = output_dir.join("logs");
    fs::create_dir_all(&log_dir)?;
    
    let filename = if name.len() > 50 {
        format!("{:x}_prompt.txt", hash)
    } else {
        format!("{}_prompt.txt", name)
    };
    
    let log_path = log_dir.join(filename);
    fs::write(log_path, format!("Prompt:\n{}\n\nContext:\n{}", prompt, context))?;
    Ok(())
}

pub fn log_llm_response(output_dir: &Path, prefix: &str, content: &str) -> std::io::Result<()> {
    let logs_dir = output_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;
    
    let filename = format!("{}_{}.txt", prefix, Local::now().format("%Y%m%d_%H%M%S"));
    let mut file = File::create(logs_dir.join(&filename))?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn log_tokens(
    output_dir: &Path,
    operation: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
) -> std::io::Result<()> {
    let total_tokens = prompt_tokens + completion_tokens;
    
    // Log to console
    info!(
        "📊 {} tokens - Prompt: {}, Completion: {}, Total: {}", 
        operation,
        prompt_tokens,
        completion_tokens,
        total_tokens
    );

    // Update metadata
    crate::update_metadata(
        output_dir,
        &format!("{} Token Usage", operation),
        &format!("Prompt tokens: {}, Completion tokens: {}, Total tokens: {}", 
            prompt_tokens, 
            completion_tokens,
            total_tokens
        )
    )?;

    // Only create a token usage log file in debug mode
    #[cfg(debug_assertions)]
    {
        let logs_dir = output_dir.join("logs");
        fs::create_dir_all(&logs_dir)?;
        
        let filename = format!("{}_token_usage_{}.txt", operation, Local::now().format("%Y%m%d_%H%M%S"));
        let mut file = File::create(logs_dir.join(&filename))?;
        file.write_all(format!(
            "Operation: {}\nPrompt tokens: {}\nCompletion tokens: {}\nTotal tokens: {}\nTimestamp: {}", 
            operation,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ).as_bytes())?;
    }

    Ok(())
}

pub fn log_llm_output(
    output_dir: &Path, 
    operation: &str, 
    content: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    token_tracker: Option<&TokenTracker>,
) -> std::io::Result<()> {
    // Log the full response to a file only
    log_llm_response(output_dir, operation, content)?;
    
    // Log a warning if prompt tokens are zero
    if prompt_tokens == 0 {
        tracing::warn!("{} has 0 prompt tokens. This may indicate an issue with token counting from the LLM service.", operation);
    }
    
    // Update token tracker if provided
    if let Some(tracker) = token_tracker {
        tracker.add_prompt_tokens(prompt_tokens as usize);
        tracker.add_completion_tokens(completion_tokens as usize);
    }
    
    // Calculate total tokens once
    let total_tokens = prompt_tokens + completion_tokens;
    
    // Log token usage in a concise format
    info!(
        "📊 {} tokens - Prompt: {}, Completion: {}, Total: {}", 
        operation,
        prompt_tokens,
        completion_tokens,
        total_tokens
    );

    // Update metadata with token usage
    crate::update_metadata(
        output_dir,
        &format!("{} Token Usage", operation),
        &format!("Prompt tokens: {}, Completion tokens: {}, Total tokens: {}", 
            prompt_tokens, 
            completion_tokens,
            total_tokens
        )
    )
}

pub fn log_with_tracker(
    output_dir: &Path,
    operation: &str,
    content: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    token_tracker: &TokenTracker,
) -> std::io::Result<()> {
    log_llm_output(
        output_dir,
        operation,
        content,
        prompt_tokens,
        completion_tokens,
        Some(token_tracker),
    )
}

/// Cleans up old log files in the output directory
/// 
/// This function removes log files older than the specified retention period,
/// while keeping essential files for debugging and recovery.
/// 
/// # Arguments
/// 
/// * `output_dir` - The output directory containing the logs
/// * `retention_days` - Number of days to keep logs (default: 7)
/// * `keep_essential` - Whether to keep essential logs regardless of age
pub fn cleanup_logs(output_dir: &Path, retention_days: u64, keep_essential: bool) -> std::io::Result<usize> {
    let logs_dir = output_dir.join("logs");
    if !logs_dir.exists() {
        return Ok(0);
    }
    
    let retention_duration = Duration::from_secs(retention_days * 24 * 60 * 60);
    let now = std::time::SystemTime::now();
    let mut removed_count = 0;
    
    if let Ok(entries) = std::fs::read_dir(&logs_dir) {
        for entry in entries.filter_map(|r| r.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            
            // Skip essential files if keep_essential is true
            if keep_essential {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        // Keep chapter and scene files as they're needed for resuming generation
                        if file_name_str.starts_with("chapter_") && file_name_str.ends_with(".md") {
                            continue;
                        }
                        // Keep the most recent generation files for each chapter/scene
                        if (file_name_str.starts_with("chapter_generation_") || 
                            file_name_str.starts_with("scene_generation_")) && 
                           !file_name_str.contains("token_usage") {
                            continue;
                        }
                    }
                }
            }
            
            // Check file age
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > retention_duration {
                            if let Err(e) = std::fs::remove_file(&path) {
                                tracing::warn!("Failed to remove old log file {}: {}", path.display(), e);
                            } else {
                                removed_count += 1;
                                tracing::debug!("Removed old log file: {}", path.display());
                            }
                        }
                    }
                }
            }
        }
    }
    
    tracing::info!("Cleaned up {} old log files from {}", removed_count, logs_dir.display());
    Ok(removed_count)
} 