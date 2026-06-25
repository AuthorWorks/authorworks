//! LLM access for the book generator.
//!
//! All inference is routed through the homelab LiteLLM gateway via the shared
//! `homelab-inference` client (OpenAI-compatible), collapsing the previous
//! hand-rolled anthropic / async-openai / ollama provider clients into one.
//! The langchain `LLM` trait surface is preserved through a thin adapter so the
//! existing prompt chains and call sites are unchanged; provider selection now
//! always resolves to the gateway (Langfuse logging + fallback ladder + per-app
//! virtual-key budgets come for free).

use crate::config::Config;
use crate::error::Result;
use async_trait::async_trait;
use futures_core::Stream;
use homelab_inference::LlmConfig;
use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::{GenerateResult, LLMError};
use langchain_rust::schemas::{Message, MessageType, StreamData};
use std::pin::Pin;

/// Error type for LLM operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Default max tokens for a single completion when the caller doesn't specify.
const DEFAULT_MAX_TOKENS: u32 = 32000;

/// Build the shared gateway client from app config, preserving the existing
/// `OPENAI_API_BASE` / `OPENAI_API_KEY` / `MODEL` env wiring.
fn gateway_config(config: &Config) -> LlmConfig {
    LlmConfig {
        base_url: config.openai_api_base.trim_end_matches('/').to_string(),
        model: config.model.clone(),
        api_key: Some(config.openai_api_key.clone()).filter(|k| !k.is_empty()),
    }
}

/// Split a langchain message list into (system, user) prompt strings for the
/// OpenAI-compatible chat call. System messages are concatenated into the
/// system prompt; everything else (human/AI/tool) folds into the user prompt.
fn split_messages(messages: &[Message]) -> (String, String) {
    let mut system = String::new();
    let mut user = String::new();
    for m in messages {
        let bucket = match m.message_type {
            MessageType::SystemMessage => &mut system,
            _ => &mut user,
        };
        if !bucket.is_empty() {
            bucket.push('\n');
        }
        bucket.push_str(&m.content);
    }
    (system, user)
}

/// langchain `LLM` adapter backed by the homelab inference gateway.
#[derive(Clone)]
pub struct HomelabLLM {
    cfg: LlmConfig,
    max_tokens: u32,
}

#[async_trait]
impl LLM for HomelabLLM {
    async fn generate(&self, messages: &[Message]) -> std::result::Result<GenerateResult, LLMError> {
        let (system, user) = split_messages(messages);
        let text = self
            .cfg
            .chat_with(&system, &user, 0.7, self.max_tokens)
            .await
            .map_err(|e| LLMError::OtherError(e.to_string()))?;
        // The gateway client returns text only; token usage is logged in
        // Langfuse rather than surfaced here. Consumers default missing counts
        // to zero.
        Ok(GenerateResult { generation: text, tokens: None })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> std::result::Result<
        Pin<Box<dyn Stream<Item = std::result::Result<StreamData, LLMError>> + Send>>,
        LLMError,
    > {
        Err(LLMError::OtherError("Streaming not supported for this LLM".to_string()))
    }
}

/// Client for LLM API interactions (native, non-langchain call path).
pub struct Client {
    cfg: LlmConfig,
}

/// Response from LLM generation
pub struct GenerationResponse {
    pub text: String,
    pub usage: Option<TokenUsage>,
}

/// Token usage information
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

/// Create a client for LLM API interactions, resolved from the environment
/// (defaults to the in-cluster gateway).
pub fn create_client() -> std::result::Result<Client, Error> {
    Ok(Client { cfg: LlmConfig::from_env() })
}

impl Client {
    /// Generate text using the specified model
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
    ) -> std::result::Result<GenerationResponse, Error> {
        self.generate_with_options(model, prompt, None).await
    }

    /// Generate text using the specified model with custom options
    pub async fn generate_with_options(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> std::result::Result<GenerationResponse, Error> {
        let mut cfg = self.cfg.clone();
        if !model.is_empty() {
            cfg.model = model.to_string();
        }
        let max_tokens = max_tokens.map(|t| t as u32).unwrap_or(DEFAULT_MAX_TOKENS);
        let text = cfg
            .chat_with("", prompt, 0.7, max_tokens)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;
        Ok(GenerationResponse { text, usage: None })
    }
}

/// Build the langchain `LLM` used by the prompt chains. Always routes through
/// the gateway; `config.llm_provider` is retained for compatibility but no
/// longer selects a distinct client.
pub fn create_llm(config: &Config) -> Result<Box<dyn langchain_rust::language_models::llm::LLM>> {
    Ok(Box::new(HomelabLLM { cfg: gateway_config(config), max_tokens: DEFAULT_MAX_TOKENS }))
}

pub fn frame_system_prompt(context: &str) -> String {
    const PROMPT_PREFIX: &str = "You are an AI assistant tasked with generating a book. Your role is to create engaging and coherent content based on the following context:\n\n";
    const PROMPT_SUFFIX: &str = "\n\nPlease ensure that your responses are creative, consistent with the given context, and follow proper narrative structure. Be mindful of character development, plot progression, and thematic elements throughout the book generation process.";

    let mut prompt = String::with_capacity(PROMPT_PREFIX.len() + context.len() + PROMPT_SUFFIX.len());
    prompt.push_str(PROMPT_PREFIX);
    prompt.push_str(context);
    prompt.push_str(PROMPT_SUFFIX);
    prompt
}

/// Check if the API is available
pub async fn api_available() -> bool {
    // For now, just return true. A real implementation would ping the gateway.
    true
}

/// Generate text using the LLM
pub async fn generate(
    model: &str,
    prompt: &str,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<String> {
    let client = create_client()
        .map_err(|e| crate::error::BookGeneratorError::LLMError(e.to_string()))?;

    let response = client
        .generate(model, prompt)
        .await
        .map_err(|e| crate::error::BookGeneratorError::LLMError(e.to_string()))?;

    if let Some(usage) = response.usage {
        token_tracker.add_prompt_tokens(usage.prompt_tokens);
        token_tracker.add_completion_tokens(usage.completion_tokens);
    }

    Ok(response.text)
}
