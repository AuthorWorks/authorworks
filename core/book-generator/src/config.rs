use serde::{Deserialize, Serialize};
use crate::error::{Result, BookGeneratorError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm_provider: String,
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub model: String,
    pub genre: String,
    pub writing_style: String,
    pub target_audience: String,
    pub max_chapters: usize,
    pub max_scenes_per_chapter: usize,
    /// Maximum length of content to include in context for next scene generation
    /// Higher values provide more context but use more tokens
    /// For Claude 3.7 Sonnet (200K tokens) and GPT-4 Turbo (128K tokens),
    /// we can safely use larger context windows, but we'll default to a conservative value
    /// that works well across all supported models including GPT-3.5 Turbo (16K tokens)
    pub max_content_length: usize,
    /// Flag to indicate whether to generate all components automatically
    pub auto_generate: bool,
    /// Duration in seconds for which temporary summaries are considered valid in cache
    /// Set to 0 to disable cache expiration (summaries will be valid indefinitely)
    pub summary_cache_duration: u64,
    /// Number of days to keep log files before automatic cleanup
    /// Set to 0 to disable automatic cleanup
    pub log_retention_days: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let get_env_or_default = |key: &str, default: &str| -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        };

        let parse_env_or_default = |key: &str, default: usize| -> usize {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let parse_u64_env_or_default = |key: &str, default: u64| -> u64 {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let parse_bool_env_or_default = |key: &str, default: bool| -> bool {
            std::env::var(key)
                .ok()
                .and_then(|v| v.to_lowercase().parse::<bool>().ok())
                .unwrap_or(default)
        };

        let llm_provider = get_env_or_default("LLM_PROVIDER", "ollama");
        
        // Only require API keys for the selected provider
        let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        
        // Validate that the required API key is present for the selected provider
        match llm_provider.as_str() {
            "openai" if openai_api_key.is_empty() => {
                return Err(BookGeneratorError::MissingEnvVar("OPENAI_API_KEY".to_string()));
            }
            "anthropic" if anthropic_api_key.is_empty() => {
                return Err(BookGeneratorError::MissingEnvVar("ANTHROPIC_API_KEY".to_string()));
            }
            // Ollama doesn't require an API key
            _ => {}
        }
        
        // Default model based on provider
        let default_model = match llm_provider.as_str() {
            "openai" => "gpt-4o",
            "anthropic" => "claude-sonnet-4-20250514",
            "ollama" => "deepseek-coder-v2:16b",
            _ => "deepseek-coder-v2:16b",
        };
        
        Ok(Self {
            llm_provider,
            openai_api_key,
            anthropic_api_key,
            model: get_env_or_default("MODEL", default_model),
            genre: get_env_or_default("GENRE", "Science Fiction"),
            writing_style: get_env_or_default("WRITING_STYLE", "Third-person limited, present tense"),
            target_audience: get_env_or_default("TARGET_AUDIENCE", "Young Adult"),
            max_chapters: parse_env_or_default("MAX_CHAPTERS", 20),
            max_scenes_per_chapter: parse_env_or_default("MAX_SCENES_PER_CHAPTER", 3),
            max_content_length: parse_env_or_default("MAX_CONTENT_LENGTH", 17000),
            auto_generate: parse_bool_env_or_default("AUTO_GENERATE", false),
            summary_cache_duration: parse_u64_env_or_default("SUMMARY_CACHE_DURATION", 86400), // Default: 24 hours
            log_retention_days: parse_u64_env_or_default("LOG_RETENTION_DAYS", 7), // Default: 7 days
        })
    }

    /// Get the model to use for a specific phase
    pub fn get_model_for_phase(&self, _phase: &str) -> &str {
        &self.model
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm_provider: "anthropic".to_string(),
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            model: "claude-3-7-sonnet-20250219".to_string(),
            genre: "Science Fiction".to_string(),
            writing_style: "Third-person limited, present tense".to_string(),
            target_audience: "Young Adult".to_string(),
            max_chapters: 42,
            max_scenes_per_chapter: 3,
            max_content_length: 17000,
            auto_generate: false,
            summary_cache_duration: 86400, // Default: 24 hours
            log_retention_days: 7, // Default: 7 days
        }
    }
}
