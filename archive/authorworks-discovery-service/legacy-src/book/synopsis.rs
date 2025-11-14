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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Synopsis {
    pub content: String,
}

impl Synopsis {
    pub async fn generate_or_input(
        (title, output_dir): (String, &std::path::Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &TokenTracker,
    ) -> Result<Self> {
        if let Some(content) = crate::read_metadata_section(output_dir, "Synopsis") {
            println!("Existing synopsis found. Do you want to use it? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing synopsis from metadata.");
                return Ok(Self { content });
            }
        }

        let result = if config.auto_generate {
            info!("Auto-generating synopsis...");
            Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
        } else {
            println!("Do you want to input the synopsis manually? (Y/n)");
            let manual_input = crate::utils::input::get_user_input("")?.to_lowercase();
            
            if manual_input != "n" {
                let content = get_multiline_input("Enter synopsis:")?;
                Self { content }
            } else {
                info!("Sending synopsis generation prompt to LLM");
                let result = Self::ai_generate(&title, context, config, output_dir, token_tracker).await?;
                info!("Received response from LLM");
                info!("Raw LLM output: {}", result.content);
                result
            }
        };

        context.add_to_history(format!("Synopsis:\n{}", result.content));
        crate::update_metadata(output_dir, "Synopsis", &result.content)?;
        Ok(result)
    }

    async fn ai_generate(title: &str, context: &Context, config: &Config, output_dir: &Path, token_tracker: &crate::utils::logging::TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::synopsis();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        let base_context = format!(
            "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nCharacters: {}",
            title,
            context.braindump.content,
            context.genre.name,
            context.genre.description,
            context.style.description,
            context.characters,
        );

        log_prompt(
            output_dir,
            "synopsis_generation",
            &prompt.template(),
            &base_context,
        )?;

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => &context.braindump.content,
            "genre" => &format!("{} - {}", context.genre.name, context.genre.description),
            "style" => &context.style.description,
            "characters" => &context.characters.to_string()
        }).await?;

        crate::log_with_tracker(
            output_dir,
            "synopsis_generation",
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

impl std::fmt::Display for Synopsis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}