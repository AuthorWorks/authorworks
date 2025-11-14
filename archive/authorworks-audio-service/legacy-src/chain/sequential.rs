use async_trait::async_trait;
use langchain_rust::{
    chain::{Chain, ChainError},
    prompt::{PromptTemplate, FormatPrompter},
    schemas::Message,
    language_models::{LLMError, GenerateResult, TokenUsage},
};
use std::collections::HashMap;
use serde_json::Value;

pub struct PromptChain {
    prompts: Vec<PromptTemplate>,
}

impl PromptChain {
    #[allow(dead_code)]
    pub fn new(prompts: Vec<PromptTemplate>) -> Self {
        Self { prompts }
    }
}

#[async_trait]
impl Chain for PromptChain {
    async fn call(&self, input: HashMap<String, Value>) -> Result<GenerateResult, ChainError> {
        let mut current_input = input;
        let mut final_generation = String::new();
        let mut total_tokens = TokenUsage::default();

        for prompt in &self.prompts {
            let formatted_prompt = prompt.format_prompt(current_input.clone())?;
            let messages = vec![Message::new_human_message(formatted_prompt.to_string())];
            let llm = crate::llm::create_llm(&crate::config::Config::default())
                .map_err(|e| ChainError::LLMError(LLMError::OtherError(e.to_string())))?;
            let response = llm.generate(&messages).await?;
            
            final_generation.push_str(&response.generation);
            if let Some(tokens) = response.tokens {
                total_tokens = TokenUsage::new(
                    total_tokens.prompt_tokens + tokens.prompt_tokens,
                    total_tokens.completion_tokens + tokens.completion_tokens
                );
            }
            
            current_input.insert("output".to_string(), Value::String(response.generation));
        }

        Ok(GenerateResult {
            generation: final_generation,
            tokens: Some(total_tokens),
        })
    }
}