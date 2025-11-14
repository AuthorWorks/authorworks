mod anthropic;
mod openai;
mod ollama;

use crate::config::Config;
use crate::error::{Result, BookGeneratorError};
use langchain_rust::llm::openai::{OpenAI, OpenAIConfig};
use crate::llm::anthropic::AnthropicLLM;
use ::anthropic::client;
use ::anthropic::config::AnthropicConfig;
use ::anthropic::types::{Message, Role, ContentBlock, MessagesRequestBuilder};

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

/// Client for LLM API interactions
pub struct Client {
    anthropic: Option<client::Client>,
    _openai: Option<reqwest::Client>,
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

/// Create a client for LLM API interactions
pub fn create_client() -> std::result::Result<Client, Error> {
    // Create Anthropic client
    let anthropic = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) if !key.is_empty() => {
            match AnthropicConfig::new() {
                Ok(cfg) => {
                    match client::Client::try_from(cfg) {
                        Ok(client) => Some(client),
                        Err(e) => return Err(Error::ClientError(format!("Failed to create Anthropic client: {}", e))),
                    }
                },
                Err(e) => return Err(Error::ConfigError(format!("Failed to create Anthropic config: {}", e))),
            }
        },
        _ => None,
    };
    
    // Create OpenAI client
    let openai = Some(reqwest::Client::new());
    
    Ok(Client {
        anthropic,
        _openai: openai,
    })
}

impl Client {
    /// Generate text using the specified model
    pub async fn generate(&self, model: &str, prompt: &str) -> std::result::Result<GenerationResponse, Error> {
        self.generate_with_options(model, prompt, None).await
    }
    
    /// Generate text using the specified model with custom options
    pub async fn generate_with_options(&self, model: &str, prompt: &str, max_tokens: Option<usize>) -> std::result::Result<GenerationResponse, Error> {
        // For now, just use Anthropic if available
        if let Some(client) = &self.anthropic {
            let message = Message {
                role: Role::User,
                content: vec![ContentBlock::Text { text: prompt.to_string() }],
            };
            
            // Create the request builder with a proper let binding
            let mut request_builder = MessagesRequestBuilder::default();
            let request_builder = request_builder
                .messages(vec![message])
                .model(model);
                
            // Apply max_tokens if specified, otherwise use default
            let request_builder = if let Some(tokens) = max_tokens {
                request_builder.max_tokens(tokens)
            } else {
                request_builder.max_tokens(32000usize)
            };
            
            // Build the request
            let request = request_builder
                .build()
                .map_err(|e| Error::ApiError(e.to_string()))?;
                
            let response = client.messages(request)
                .await
                .map_err(|e| Error::ApiError(e.to_string()))?;
                
            // Extract text from response
            let text = response.content.iter()
                .filter_map(|block| {
                    if let ContentBlock::Text { text } = block {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<String>();
                
            // Extract token usage
            let usage = Some(TokenUsage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
            });
            
            Ok(GenerationResponse { text, usage })
        } else {
            // Fallback to a simple response for testing
            Ok(GenerationResponse {
                text: "This is a placeholder response since no LLM client is available.".to_string(),
                usage: None,
            })
        }
    }
}

pub fn create_llm(config: &Config) -> Result<Box<dyn langchain_rust::language_models::llm::LLM>> {
    match config.llm_provider.as_str() {
        "openai" => {
            let model = if config.model.is_empty() { "o3" } else { &config.model };
            let openai = OpenAI::default()
                .with_config(
                    OpenAIConfig::default()
                        .with_api_key(&config.openai_api_key)
                )
                .with_model(model);
            Ok(Box::new(openai))
        }
        "anthropic" => {
            let anthropic = AnthropicLLM::new(&config.model)
                .map_err(|e| BookGeneratorError::Other(e.to_string()))?;
            Ok(Box::new(anthropic))
        }
        "ollama" => {
            let ollama = crate::llm::ollama::OllamaLLM::new(config)?;
            Ok(Box::new(ollama))
        }
        _ => Err(BookGeneratorError::UnsupportedLLMProvider(
            config.llm_provider.clone(),
        )),
    }
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
    // For now, just return true
    // In a real implementation, this would check if the API is available
    // by making a small request or checking a status endpoint
    true
}

/// Generate text using the LLM
pub async fn generate(
    model: &str,
    prompt: &str,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<String> {
    // Create a client
    let client = create_client().map_err(|e| crate::error::BookGeneratorError::LLMError(e.to_string()))?;
    
    // Generate text
    let response = client.generate(model, prompt)
        .await
        .map_err(|e| crate::error::BookGeneratorError::LLMError(e.to_string()))?;
    
    // Track tokens
    if let Some(usage) = response.usage {
        token_tracker.add_prompt_tokens(usage.prompt_tokens);
        token_tracker.add_completion_tokens(usage.completion_tokens);
    }
    
    Ok(response.text)
}