use async_trait::async_trait;
use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::{GenerateResult, TokenUsage};
use langchain_rust::schemas::{Message, MessageType, StreamData};
use anthropic::client::Client as AnthropicClient;
use anthropic::types::{Message as AnthropicMessage, Role, ContentBlock, MessagesRequestBuilder};
use anthropic::config::AnthropicConfig;
use std::pin::Pin;
use futures_core::Stream;
use langchain_rust::language_models::LLMError;
use std::time::Duration;
use tracing::warn;

pub struct AnthropicLLM {
    client: AnthropicClient,
    model: String,
    max_retries: usize,
}

impl AnthropicLLM {
    pub fn new(model: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cfg = AnthropicConfig::new()?;
        let client = AnthropicClient::try_from(cfg)?;
        Ok(Self { 
            client, 
            model: model.to_string(),
            max_retries: 5, // Default to 5 retries
        })
    }
    
    // Helper function to implement exponential backoff retry
    async fn retry_with_backoff<F, Fut, T>(
        &self,
        operation: F,
    ) -> std::result::Result<T, LLMError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, LLMError>>,
    {
        let mut retries = 0;
        let mut delay = Duration::from_millis(1000); // Start with 1 second delay
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    let err_str = err.to_string();
                    if (err_str.contains("overloaded") || 
                        err_str.contains("Overloaded") || 
                        err_str.contains("overloaded_error")) && 
                       retries < self.max_retries {
                        retries += 1;
                        warn!("Anthropic API overloaded. Retry {}/{} after {:?}", 
                              retries, self.max_retries, delay);
                        tokio::time::sleep(delay).await;
                        // Exponential backoff: double the delay for next retry
                        delay = delay.mul_f32(2.0);
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
}

impl Clone for AnthropicLLM {
    fn clone(&self) -> Self {
        let cfg = AnthropicConfig::new()
            .expect("Failed to create AnthropicConfig");
        Self {
            client: AnthropicClient::try_from(cfg).expect("Failed to create AnthropicClient"),
            model: self.model.clone(),
            max_retries: self.max_retries,
        }
    }
}

#[async_trait]
impl LLM for AnthropicLLM {
    async fn generate(&self, messages: &[Message]) -> std::result::Result<GenerateResult, LLMError> {
        let anthropic_messages: Vec<AnthropicMessage> = messages.iter().map(|m| AnthropicMessage {
            role: match m.message_type {
                MessageType::SystemMessage => Role::User,
                MessageType::HumanMessage => Role::User,
                MessageType::AIMessage => Role::Assistant,
                _ => Role::User,
            },
            content: vec![ContentBlock::Text { text: m.content.clone() }],
        }).collect();

        let request = MessagesRequestBuilder::default()
            .messages(anthropic_messages)
            .model(&self.model)
            .max_tokens(32000usize)
            .build()
            .map_err(|e| LLMError::OtherError(e.to_string()))?;

        // Use the retry mechanism for the API call
        let response = self.retry_with_backoff(|| async {
            self.client.messages(request.clone())
                .await
                .map_err(|e| LLMError::OtherError(e.to_string()))
        }).await?;

        // Use filter_map to directly collect non-empty text blocks
        let completion = response.content.iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<String>();

        // Extract token usage from the response
        let prompt_tokens = response.usage.input_tokens;
        let completion_tokens = response.usage.output_tokens;

        Ok(GenerateResult {
            generation: completion,
            tokens: Some(TokenUsage::new(prompt_tokens as u32, completion_tokens as u32)),
        })
    }

    async fn stream(&self, _messages: &[Message]) -> std::result::Result<Pin<Box<dyn Stream<Item = std::result::Result<StreamData, LLMError>> + Send>>, LLMError> {
        Err(LLMError::OtherError("Streaming not supported for this LLM".to_string()))
    }
}