use crate::book::Context;
use crate::config::Config;
use crate::error::{Result, BookGeneratorError};
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use crate::utils::input::get_multiline_input;
use crate::utils::logging::log_prompt;
use langchain_rust::prompt::PromptFromatter;
use tracing::info;
use std::path::Path;
use std::fmt;
use crate::utils::logging::TokenTracker;
use crate::utils::logging::log_with_tracker;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Genre {
    pub name: String,
    pub description: String,
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

impl Genre {
    pub async fn generate_or_input(
        (title, output_dir): (String, &std::path::Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &TokenTracker,
    ) -> Result<Self> {
        if let Some(content) = crate::read_metadata_section(output_dir, "Genre") {
            println!("Existing genre found. Do you want to use it? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing genre from metadata.");
                return Self::from_string(&content);
            }
        }

        let result = if config.auto_generate {
            info!("Auto-generating genre...");
            Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
        } else {
            println!("Do you want to input the genre manually? (Y/n)");
            let manual_input = crate::utils::input::get_user_input("")?.to_lowercase();

            if manual_input != "n" {
                let name = get_multiline_input("Enter genre name:")?;
                let description = get_multiline_input("Enter genre description:")?;
                Self { name, description }
            } else {
                Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
            }
        };

        context.add_to_history(format!("Genre: {}", result));
        crate::update_metadata(output_dir, "Genre", &format!("{}: {}", result.name, result.description))?;
        Ok(result)
    }

    async fn ai_generate(title: &str, context: &Context, config: &Config, output_dir: &Path, token_tracker: &TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::genre();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        let base_context = format!(
            "Title: {}\nBraindump: {}",
            title,
            context.braindump.content,
        );

        // Add logging for genre generation
        log_prompt(
            output_dir,
            "genre_generation",
            &prompt.template(),
            &base_context,
        )?;

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => &context.braindump.content
        }).await?;

        log_with_tracker(
            output_dir,
            "genre_generation",
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        Self::parse_from_llm(&output.generation)
    }

    fn from_string(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() == 2 {
            Ok(Self {
                name: parts[0].trim().to_string(),
                description: parts[1].trim().to_string(),
            })
        } else {
            // Handle plain genre names without description
            let name = s.trim().to_string();
            if name.is_empty() {
                Ok(Self::default())
            } else {
                Ok(Self {
                    name: name.clone(),
                    description: format!("A {} story", name.to_lowercase()),
                })
            }
        }
    }

    fn parse_from_llm(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() == 2 {
            Ok(Self {
                name: parts[0].trim().to_string(),
                description: parts[1].trim().to_string(),
            })
        } else {
            // Handle plain genre names without description
            let name = s.trim().to_string();
            if name.is_empty() {
                Ok(Self::default())
            } else {
                Ok(Self {
                    name: name.clone(),
                    description: format!("A {} story", name.to_lowercase()),
                })
            }
        }
    }
}
