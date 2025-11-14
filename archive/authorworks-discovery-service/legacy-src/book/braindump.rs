use crate::book::Context;
use crate::config::Config;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use crate::utils::input::get_multiline_input;
use crate::utils::logging::log_prompt;
use langchain_rust::prompt::PromptFromatter;
use tracing::info;
use std::path::Path;
use crate::utils::logging::TokenTracker;
use crate::utils::logging::log_with_tracker;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Braindump {
    pub content: String,
}

impl Braindump {
    pub async fn generate_or_input(
        (title, output_dir): (Option<String>, &std::path::Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &TokenTracker,
    ) -> Result<Self> {
        if let Some(content) = crate::read_metadata_section(output_dir, "Braindump") {
            println!("Existing braindump found. Do you want to use it? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing braindump from metadata.");
                return Ok(Self { content });
            }
        }

        // If auto_generate is true, skip manual input prompt
        let result = if config.auto_generate {
            info!("Auto-generating braindump...");
            let result = Self::ai_generate(title.as_deref().unwrap_or(""), config, output_dir, token_tracker).await?;
            info!("Received response from LLM");
            info!("Raw LLM output: {}", result.content);
            result
        } else {
            println!("Do you want to input the braindump manually? (Y/n)");
            let manual_input = crate::utils::input::get_user_input("")?.to_lowercase();
            
            if manual_input != "n" {
                let content = get_multiline_input("Enter braindump:")?;
                Self { content }
            } else {
                info!("Sending braindump generation prompt to LLM");
                let result = Self::ai_generate(title.as_deref().unwrap_or(""), config, output_dir, token_tracker).await?;
                info!("Received response from LLM");
                info!("Raw LLM output: {}", result.content);
                result
            }
        };

        context.add_to_history(format!("Braindump:\n{}", result.content));
        crate::update_metadata(output_dir, "Braindump", &result.content)?;
        Ok(result)
    }

    pub async fn ai_generate(title: &str, config: &Config, output_dir: &Path, token_tracker: &TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::braindump();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        // Add logging for braindump generation
        log_prompt(
            output_dir,
            "braindump_generation",
            &prompt.template(),
            title,
        )?;

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title
        }).await?;

        log_with_tracker(
            output_dir,
            "braindump_generation",
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        Ok(Self {
            content: output.generation.trim().to_string(),
        })
    }
}