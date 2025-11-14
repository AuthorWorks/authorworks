use crate::book::{Context, ChapterOutline};
use crate::book::outline::SceneOutline;
use crate::book::content::Content;
use crate::book::scene::Scene;
use crate::config::Config;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::utils::logging::log_with_tracker;
use langchain_rust::prompt::PromptTemplate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporarySummary {
    pub content: String,
    #[serde(skip)]
    pub last_context_hash: Option<u64>,
    #[serde(with = "timestamp_serde")]
    pub created_at: std::time::SystemTime,
}

// Custom serialization for SystemTime
mod timestamp_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?
            .as_secs();
        serializer.serialize_u64(timestamp)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(timestamp))
    }
}

impl TemporarySummary {
    // Helper function to compute a hash of the context
    fn compute_context_hash(context_str: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // For very large contexts, only hash a representative sample to improve performance
        // while still maintaining a good hash distribution
        if context_str.len() > 2000 {
            // Hash the length first (important for uniqueness)
            context_str.len().hash(&mut hasher);
            
            // Hash the first 1000 characters (beginning context)
            let start = &context_str[..1000];
            start.hash(&mut hasher);
            
            // Hash the last 1000 characters (most recent/relevant context)
            let end = &context_str[context_str.len() - 1000..];
            end.hash(&mut hasher);
        } else {
            // For smaller contexts, hash the entire string
            context_str.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    // Helper method to check cache and return cached summary if valid
    fn check_cache(
        output_dir: &Path, 
        prefix: &str, 
        number: usize, 
        context_hash: u64,
        config: &Config,
    ) -> std::io::Result<Option<Self>> {
        let cache_dir = output_dir.join("cache");
        let cache_file = cache_dir.join(format!("summary_{}_{}.json", prefix, number));
        
        if !cache_file.exists() {
            return Ok(None);
        }
        
        let json = std::fs::read_to_string(cache_file)?;
        match serde_json::from_str::<Self>(&json) {
            Ok(summary) => {
                // Check if the context hash matches
                if summary.last_context_hash == Some(context_hash) {
                    // Check if the cache has expired (if cache duration is enabled)
                    if config.summary_cache_duration > 0 {
                        if let Ok(elapsed) = summary.created_at.elapsed() {
                            if elapsed.as_secs() > config.summary_cache_duration {
                                tracing::info!("Cached summary for {} has expired", prefix);
                                return Ok(None);
                            }
                        }
                    }
                    
                    tracing::info!("Using cached temporary summary for {}", prefix);
                    return Ok(Some(summary));
                }
                Ok(None)
            },
            Err(e) => {
                tracing::warn!("Failed to parse cached summary: {}", e);
                Ok(None)
            }
        }
    }

    // Helper method to process LLM response and create a new summary
    async fn generate_summary(
        base_context: &str,
        prompt: PromptTemplate,
        config: &Config,
        output_dir: &Path,
        operation_name: &str,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        let context_hash = Self::compute_context_hash(base_context);
        
        let llm = crate::llm::create_llm(config)?;
        let chain = LLMChainBuilder::new()
            .prompt(prompt)
            .llm(llm)
            .build()?;

        let output = chain.call(langchain_rust::prompt_args!{
            "context" => base_context
        }).await?;

        log_with_tracker(
            output_dir,
            operation_name,
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        let result = Self { 
            content: output.generation.trim().to_string(),
            last_context_hash: Some(context_hash),
            created_at: std::time::SystemTime::now(),
        };
        
        Ok(result)
    }

    pub async fn generate_chapter(
        title: &str,
        context: &Context,
        config: &Config,
        previous_chapters: &[ChapterOutline],
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        if previous_chapters.len() >= context.outline.chapters.len() {
            return Err(crate::error::BookGeneratorError::Generation(
                format!("Attempted to generate chapter {} when the outline only contains {} chapters", 
                    previous_chapters.len() + 1, 
                    context.outline.chapters.len()
                )
            ));
        }
        
        if previous_chapters.len() + 1 > config.max_chapters {
            return Err(crate::error::BookGeneratorError::Generation(
                format!("Attempted to generate chapter {} when the maximum is set to {} chapters", 
                    previous_chapters.len() + 1, 
                    config.max_chapters
                )
            ));
        }

        let story_progress = previous_chapters.iter()
            .map(|ch| ch.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        let base_context = format!(
            "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nCharacters: {}\nSynopsis: {}\nBook Outline: {}\n\nPrevious Chapters:\n{}\n\nHere is where we continue the story...",
            title,
            context.braindump.content,
            context.genre.name,
            context.genre.description,
            context.style.description,
            context.characters,
            context.synopsis.content,
            context.outline,
            if story_progress.is_empty() {
                "Starting the first chapter...".to_string()
            } else {
                story_progress
            }
        );
        
        // Compute hash of the context
        let context_hash = Self::compute_context_hash(&base_context);
        let chapter_num = previous_chapters.len() + 1;
        let prefix = "chapter";
        
        // Check cache first
        if let Some(cached) = Self::check_cache(output_dir, prefix, chapter_num, context_hash, config)? {
            return Ok(cached);
        }

        // Generate new summary
        let prompt = Prompts::temporary_summary_chapter();
        let operation_name = &format!("temporary_summary_chapter_{}", chapter_num);
        
        let result = Self::generate_summary(
            &base_context, 
            prompt, 
            config, 
            output_dir, 
            operation_name, 
            token_tracker
        ).await?;
        
        // Save the summary to cache
        result.save_to_cache(output_dir, prefix, chapter_num)?;
        
        crate::update_metadata(
            output_dir,
            &format!("Chapter {} Summary", chapter_num),
            &result.content
        )?;

        Ok(result)
    }

    pub async fn generate_scene(
        title: &str,
        context: &Context,
        config: &Config,
        chapter_outline: &ChapterOutline,
        previous_scenes: &[Scene],
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        let scene_progress = if previous_scenes.is_empty() {
            "Starting the first scene...".to_string()
        } else {
            let scenes = previous_scenes.iter()
                .map(|sc| format!("Scene {}: {}\n{}", 
                    sc.outline.number, 
                    sc.title, 
                    sc.outline.description
                ))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("Previous scenes in this chapter:\n\n{}", scenes)
        };

        let chapter_outlines = context.outline.chapters.iter()
            .take_while(|ch| ch.chapter_number <= chapter_outline.chapter_number)
            .map(|ch| ch.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        let base_context = format!(
            "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nSynopsis: {}\nBook Outline: {}\nChapter Outlines:\n{}\nCurrent Chapter: {}\n\n{}\n\nHere is where we continue the story...",
            title,
            context.braindump.content,
            context.genre.name,
            context.genre.description,
            context.style.description,
            context.synopsis.content,
            context.outline,
            chapter_outlines,
            chapter_outline,
            scene_progress
        );
        
        // Compute hash of the context
        let context_hash = Self::compute_context_hash(&base_context);
        let scene_num = previous_scenes.len() + 1;
        let prefix = format!("scene_generation_ch{}_scene{}", chapter_outline.chapter_number, scene_num);
        
        // Check cache first
        if let Some(cached) = Self::check_cache(output_dir, &prefix, 1, context_hash, config)? {
            println!("  📋 Using cached temporary summary for scene {}.{}", chapter_outline.chapter_number, scene_num);
            return Ok(cached);
        }

        // Generate new summary
        let prompt = Prompts::temporary_summary_scene();
        let operation_name = &format!("temporary_summary_scene_ch{}_scene{}", 
            chapter_outline.chapter_number, scene_num);
        
        let result = Self::generate_summary(
            &base_context, 
            prompt, 
            config, 
            output_dir, 
            operation_name, 
            token_tracker
        ).await?;
        
        // Save the summary to cache
        result.save_to_cache(output_dir, &prefix, 1)?;
        
        crate::update_metadata(
            output_dir,
            &format!("Scene {} Summary", scene_num),
            &result.content
        )?;

        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn generate_content(
        title: &str,
        context: &Context,
        config: &Config,
        chapter_outline: &ChapterOutline,
        scene_outline: &SceneOutline,
        previous_content: &[Content],
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        let content_progress = if previous_content.is_empty() {
            "Starting the first scene content...".to_string()
        } else {
            let content = previous_content.iter()
                .enumerate()
                .map(|(i, c)| {
                    // Truncate very long content to save tokens, using configurable limit
                    let truncated_text = if c.text.len() > config.max_content_length {
                        format!("{}... (truncated for brevity)", &c.text[..config.max_content_length])
                    } else {
                        c.text.clone()
                    };
                    format!("Content for Scene {}:\n{}", i + 1, truncated_text)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("Previous content in this chapter:\n\n{}", content)
        };

        let scene_outlines = chapter_outline.scenes.iter()
            .take_while(|sc| sc.number <= scene_outline.number)
            .map(|sc| format!("Scene {}: {}\n{}", sc.number, sc.title, sc.description))
            .collect::<Vec<_>>()
            .join("\n\n");

        let base_context = format!(
            "Title: {}\nGenre: {} - {}\nStyle: {}\nSynopsis: {}\nBook Outline: {}\nChapter Outline: {}\nScene Outlines:\n{}\nCurrent Scene: {}\n\n{}\n\nHere is where we continue the story...",
            title,
            context.genre.name,
            context.genre.description,
            context.style.description,
            context.synopsis.content,
            context.outline,
            chapter_outline,
            scene_outlines,
            scene_outline,
            content_progress
        );
        
        // Compute hash of the context
        let context_hash = Self::compute_context_hash(&base_context);
        let prefix = &format!("content_ch{}_scene{}", chapter_outline.chapter_number, scene_outline.number);
        
        // Check cache first
        if let Some(cached) = Self::check_cache(output_dir, prefix, 1, context_hash, config)? {
            return Ok(cached);
        }

        // Generate new summary
        let prompt = Prompts::temporary_summary_content();
        let operation_name = &format!("temporary_summary_content_ch{}_scene{}", 
            chapter_outline.chapter_number, scene_outline.number);
        
        let result = Self::generate_summary(
            &base_context, 
            prompt, 
            config, 
            output_dir, 
            operation_name, 
            token_tracker
        ).await?;
        
        // Save the summary to cache
        result.save_to_cache(output_dir, prefix, 1)?;
        
        crate::update_metadata(
            output_dir,
            &format!("Content Summary for Scene {} in Chapter {}", 
                scene_outline.number, 
                chapter_outline.chapter_number),
            &result.content
        )?;

        Ok(result)
    }
    
    // Save the summary to a cache file
    fn save_to_cache(&self, output_dir: &Path, prefix: &str, number: usize) -> std::io::Result<()> {
        let cache_dir = output_dir.join("cache");
        
        // Only create the directory if it doesn't exist
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }
        
        let cache_file = cache_dir.join(format!("summary_{}_{}.json", prefix, number));
        
        // Use to_writer to avoid the intermediate string allocation
        let file = std::fs::File::create(cache_file)?;
        serde_json::to_writer(file, self)?;
        
        Ok(())
    }
}

impl Default for TemporarySummary {
    fn default() -> Self {
        Self {
            content: String::new(),
            last_context_hash: None,
            created_at: std::time::SystemTime::now(),
        }
    }
}