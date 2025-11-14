use async_trait::async_trait;
use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::{GenerateResult, TokenUsage};
use langchain_rust::schemas::{Message, StreamData};
use async_openai::{
    types::{
        CreateChatCompletionRequestArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestAssistantMessageArgs,
    },
    Client,
};
use async_openai::config::OpenAIConfig;
use std::pin::Pin;
use futures_core::Stream;
use langchain_rust::language_models::LLMError;

#[derive(Clone)]
pub struct OpenAILLM {
    client: Client<OpenAIConfig>,
    model: String,
}

#[async_trait]
impl LLM for OpenAILLM {
    async fn generate(&self, messages: &[Message]) -> std::result::Result<GenerateResult, LLMError> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages.iter().map(|m| {
                match m.message_type {
                    langchain_rust::schemas::MessageType::HumanMessage => {
                        ChatCompletionRequestMessage::User(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(m.content.clone())
                                .build()
                                .unwrap()
                        )
                    },
                    langchain_rust::schemas::MessageType::AIMessage => {
                        ChatCompletionRequestMessage::Assistant(
                            ChatCompletionRequestAssistantMessageArgs::default()
                                .content(m.content.clone())
                                .build()
                                .unwrap()
                        )
                    },
                    langchain_rust::schemas::MessageType::SystemMessage => {
                        ChatCompletionRequestMessage::System(
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(m.content.clone())
                                .build()
                                .unwrap()
                        )
                    },
                    _ => ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(m.content.clone())
                            .build()
                            .unwrap()
                    ),
                }
            }).collect::<Vec<_>>())
            .build()
            .map_err(|e| LLMError::OtherError(e.to_string()))?;

        let response = self.client.chat().create(request).await
            .map_err(|e| LLMError::OtherError(e.to_string()))?;
        let choice = response.choices.first().ok_or_else(|| LLMError::OtherError("Empty response".to_string()))?;

        Ok(GenerateResult {
            generation: choice.message.content.clone().unwrap_or_default(),
            tokens: response.usage.as_ref().map(|u| TokenUsage::new(u.prompt_tokens, u.completion_tokens)),
        })
    }

    async fn stream(&self, _messages: &[Message]) -> std::result::Result<Pin<Box<dyn Stream<Item = std::result::Result<StreamData, LLMError>> + Send>>, LLMError> {
        Err(LLMError::OtherError("Streaming not supported for this LLM".to_string()))
    }
}
