//! Anthropic Claude API client

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

pub struct AnthropicClient {
    client: Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<String> {
        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            system: system.map(|s| s.to_string()),
        };

        debug!("Sending request to Anthropic API");

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Anthropic API error: {} - {}",
                status,
                error_text
            ));
        }

        let response: MessageResponse = response.json().await
            .context("Failed to parse Anthropic response")?;

        let text = response.content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    Some(block.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    pub async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::StreamExt;

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            system: system.map(|s| s.to_string()),
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&serde_json::json!({
                "model": request.model,
                "max_tokens": request.max_tokens,
                "messages": request.messages,
                "system": request.system,
                "stream": true
            }))
            .send()
            .await
            .context("Failed to send streaming request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Anthropic API error: {} - {}",
                status,
                error_text
            ));
        }

        let stream = response.bytes_stream().map(|result| {
            result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    // Parse SSE events
                    let mut content = String::new();
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data != "[DONE]" {
                                if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            content.push_str(&text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(content)
                })
        });

        Ok(stream)
    }
}

#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
}

