use async_trait::async_trait;
use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::{GenerateResult, TokenUsage};
use langchain_rust::schemas::{Message, StreamData};
use ollama_rs::{Ollama, generation::completion::{request::GenerationRequest, GenerationResponse}};
use crate::config::Config;
use crate::error::Result;
use std::pin::Pin;
use futures_core::Stream;
use langchain_rust::language_models::LLMError;

#[derive(Clone)]
pub struct OllamaLLM {
    client: Ollama,
    model: String,
}

impl OllamaLLM {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            client: Ollama::default(),
            model: config.model.clone(),
        })
    }
}

#[async_trait]
impl LLM for OllamaLLM {
    async fn generate(&self, messages: &[Message]) -> std::result::Result<GenerateResult, LLMError> {
        // Estimate the total capacity needed for the prompt
        let capacity = messages.iter().map(|m| m.content.len()).sum::<usize>() + messages.len();
        let mut prompt = String::with_capacity(capacity);
        
        // Build the prompt string with fewer allocations
        for (i, m) in messages.iter().enumerate() {
            if i > 0 {
                prompt.push('\n');
            }
            prompt.push_str(&m.content);
        }
        
        let request = GenerationRequest::new(self.model.clone(), prompt);
        let response: GenerationResponse = self.client.generate(request)
            .await
            .map_err(|e| LLMError::OtherError(e.to_string()))?;

        Ok(GenerateResult {
            generation: response.response,
            tokens: Some(TokenUsage::new(
                response.prompt_eval_count.unwrap_or(0) as u32,
                response.eval_count.unwrap_or(0) as u32
            )),
        })
    }

    async fn stream(&self, _messages: &[Message]) -> std::result::Result<Pin<Box<dyn Stream<Item = std::result::Result<StreamData, LLMError>> + Send>>, LLMError> {
        Err(LLMError::OtherError("Streaming not supported for this LLM".to_string()))
    }
}