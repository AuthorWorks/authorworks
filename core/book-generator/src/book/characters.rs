use crate::book::Context;
use crate::config::Config;
use crate::error::{Result, BookGeneratorError};
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use crate::utils::input::get_multiline_input;
use tracing::info;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Characters {
    pub list: Vec<Character>,
}

impl Characters {
    pub async fn generate_or_input(
        (title, output_dir): (String, &std::path::Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        if let Some(content) = crate::read_metadata_section(output_dir, "Characters") {
            println!("Existing characters found. Do you want to use them? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing characters from metadata.");
                return Ok(Self::from_string(&content));
            }
        }

        let result = if config.auto_generate {
            info!("Auto-generating characters...");
            Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
        } else {
            println!("Do you want to input the characters manually? (Y/n)");
            let manual_input = crate::utils::input::get_user_input("")?.to_lowercase();
            
            if manual_input != "n" {
                let mut characters = Vec::new();
                loop {
                    let name = get_multiline_input("Enter character name (or press Enter to finish):")?;
                    if name.is_empty() {
                        break;
                    }
                    let description = get_multiline_input("Enter character description:")?;
                    characters.push(Character { name, description });
                }
                Self { list: characters }
            } else {
                Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
            }
        };

        context.add_to_history(format!("Characters:\n{}", result));
        crate::update_metadata(output_dir, "Characters", &result.to_string())?;
        Ok(result)
    }

    async fn ai_generate(title: &str, context: &Context, config: &Config, _output_dir: &Path, _token_tracker: &crate::utils::logging::TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::characters();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        let base_context = format!(
            "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}",
            title,
            context.braindump.content,
            context.genre.name,
            context.genre.description,
            context.style.description,
        );

        info!("Full context:\n{}", base_context);

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => &context.braindump.content,
            "genre" => &format!("{} - {}", context.genre.name, context.genre.description),
            "style" => &context.style.description
        }).await?;

        info!("Received response from LLM");
        info!("Raw LLM output: {}", output.generation);

        let result = Self::from_string(&output.generation);
        if result.list.is_empty() {
            return Err(BookGeneratorError::SerializationError(
                "Character generation produced no valid characters".to_string()
            ));
        }

        Ok(result)
    }

    fn from_string(content: &str) -> Self {
        let mut characters = Vec::new();
        for line in content.lines() {
            if let Some((name, description)) = line.split_once(':') {
                characters.push(Character {
                    name: name.trim().to_string(),
                    description: description.trim().to_string(),
                });
            }
        }
        Self { list: characters }
    }
}

impl std::fmt::Display for Characters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for character in &self.list {
            writeln!(f, "{}: {}", character.name, character.description)?;
        }
        Ok(())
    }
}