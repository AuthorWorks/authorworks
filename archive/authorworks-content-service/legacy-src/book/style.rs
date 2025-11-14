use crate::book::Context;
use crate::config::Config;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use crate::utils::input::{get_user_input, get_multiline_input};
use crate::utils::logging::log_prompt;
use tracing::info;
use std::path::Path;
use langchain_rust::prompt::PromptFromatter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub description: String,
}

impl Style {
    pub async fn generate_or_input(
        (title, output_dir): (String, &Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        if let Some(content) = crate::read_metadata_section(output_dir, "Style") {
            println!("Existing style found. Do you want to use it? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing style from metadata.");
                let result = Self { description: content };
                return Ok(result);
            }
        }

        // If auto_generate is true, skip manual input prompt
        let result = if config.auto_generate {
            info!("Auto-generating style...");
            Self::ai_generate(&title, context, config, output_dir, token_tracker).await?
        } else {
            println!("Generating style...");
            println!("1) Use default style\n2) Input manually\n3) Generate new style");
            let choice = get_user_input("Choose an option (1-3):\n")?;
            
            match choice.trim() {
                "1" => Self::default(),
                "2" => {
                    let description = get_multiline_input("Enter style description:")?;
                    Self { description }
                },
                "3" => Self::ai_generate(&title, context, config, output_dir, token_tracker).await?,
                _ => {
                    println!("Invalid choice, using default style.");
                    Self::default()
                }
            }
        };
        
        context.add_to_history(format!("Style:\n{}", result.description));
        crate::update_metadata(output_dir, "Style", &result.description)?;
        Ok(result)
    }

    async fn ai_generate(title: &str, context: &Context, config: &Config, output_dir: &Path, token_tracker: &crate::utils::logging::TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::style();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        // Add logging for style generation
        log_prompt(
            output_dir,
            "style_generation",
            &prompt.template(),
            &format!("Title: {}\nBraindump: {}\nGenre: {} - {}", 
                title, 
                context.braindump.content,
                context.genre.name,
                context.genre.description
            ),
        )?;

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => context.braindump.content.clone(),
            "genre" => format!("{} - {}", context.genre.name, context.genre.description),
        }).await?;

        crate::utils::logging::log_with_tracker(
            output_dir,
            "style_generation",
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        Ok(Self {
            description: output.generation.trim().to_string(),
        })
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            description: String::from(
                "The writing should be realistic, whimsical, creative, clever, VERY OCCASIONALLY wryly humorous, \
                wise - and most of all both aware of and sensitive to both the intricacies of human nature, and \
                the techniques used by writers to entertain and captivate their readers. Remember this guidance \
                on prose rhythm: 'This sentence has five words. Here are five more words. Five-word sentences are \
                fine. But several together become monotonous. Listen to what is happening. The writing is getting \
                boring. The sound of it drones. It's like a stuck record. The ear demands some variety. Now listen. \
                I vary the sentence length, and I create music. Music. The writing sings. It has a pleasant rhythm, \
                a lilt, a harmony. I use short sentences. And I use sentences of medium length. And sometimes, when \
                I am certain the reader is rested, I will engage him with a sentence of considerable length, a \
                sentence that burns with energy and builds with all the impetus of a crescendo, the roll of the \
                drums, the crash of the cymbals—sounds that say listen to this, it is important.' Keep this in \
                mind when generating all content, and make sure our tone is a mix of Dave Barry-esque humor and \
                a Terry Pratchett-esque sense of satire about current society and optimism for the future, with \
                a dash of Carl Hiassen's keen eye and manic florida man stories, and a large ladleful of Douglas \
                Adams amazingly esoteric and oddball descriptive language."
            )
        }
    }
}