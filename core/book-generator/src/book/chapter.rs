use crate::book::{Context, ChapterOutline};
use crate::book::scene::Scene;
use crate::config::Config;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use langchain_rust::prompt::PromptFromatter;
use crate::TemporarySummary;
use tracing::info;
use std::path::Path;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Chapter {
    pub number: usize,
    pub title: String,
    pub outline: ChapterOutline,
    #[serde(bound(deserialize = "Scene: Deserialize<'de>"))]
    pub scenes: Vec<Scene>,
    pub content: String,
}

impl Chapter {
    pub async fn generate(
        chapter_number: usize,
        title: &str,
        context: &Context,
        config: &Config,
        previous_chapters: &[ChapterOutline],
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        println!("📚 Generating chapter {} outline: \"{}\"...", chapter_number, title);
        info!("Generating chapter {} outline...", chapter_number);

        // Check if we've already generated all chapters from the outline
        if chapter_number > context.outline.chapters.len() {
            return Err(crate::error::BookGeneratorError::Generation(
                format!("Attempted to generate chapter {} when the outline only contains {} chapters",
                    chapter_number,
                    context.outline.chapters.len()
                )
            ));
        }

        // Check if we've exceeded the maximum number of chapters specified in config
        if chapter_number > config.max_chapters {
            return Err(crate::error::BookGeneratorError::Generation(
                format!("Attempted to generate chapter {} when the maximum is set to {} chapters",
                    chapter_number,
                    config.max_chapters
                )
            ));
        }

        println!("📝 Generating temporary summary for chapter {}", chapter_number);
        // Generate temporary summary first
        let temp_summary = TemporarySummary::generate_chapter(
            title,
            context,
            config,
            previous_chapters,
            output_dir,
            token_tracker,
        ).await?;

        println!("✨ Generating detailed outline for chapter {}", chapter_number);
        info!("Sending chapter outline generation prompt to LLM");

        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::chapter();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        // Format previous chapters for context
        let previous_chapters_text = if previous_chapters.is_empty() {
            "This is the first chapter of the book.".to_string()
        } else {
            let chapters = previous_chapters.iter()
                .map(|ch| ch.to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("Previous chapters in the book:\n\n{}", chapters)
        };

        // Log the chapter generation prompt
        crate::utils::logging::log_prompt(
            output_dir,
            &format!("chapter_generation_{}", chapter_number),
            &format!(
                "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nCharacters: {}\nSynopsis: {}\nBook Outline: {}\nPrevious Chapters: {}\nTemporary Summary: {}",
                title,
                context.braindump.content,
                context.genre.name, context.genre.description,
                context.style.description,
                context.characters,
                context.synopsis.content,
                context.outline,
                previous_chapters_text,
                temp_summary.content
            ),
            &prompt.template()
        )?;

        // Use only the required context for chapter generation as specified:
        // - title, braindump, genre, style, characters
        // - book synopsis
        // - book outline
        // - temporary chapter summary
        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => &context.braindump.content,
            "genre" => &format!("{} - {}", context.genre.name, context.genre.description),
            "style" => &context.style.description,
            "characters" => &context.characters,
            "synopsis" => &context.synopsis.content,
            "book_outline" => &context.outline,
            "temporary_summary" => &temp_summary.content
        }).await?;

        crate::log_with_tracker(
            output_dir,
            &format!("chapter_generation_{}", chapter_number),
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        let mut outline = ChapterOutline::parse_from_llm(&output.generation);
        outline.chapter_number = chapter_number;

        println!("✅ Successfully generated outline for chapter {}: \"{}\" with {} scenes",
                 chapter_number, title, outline.scenes.len());

        Ok(Self {
            number: chapter_number,
            title: title.to_string(),
            outline,
            scenes: Vec::new(),
            content: output.generation,
        })
    }

    #[allow(dead_code)]
    fn from_string(content: &str) -> Self {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Self::default();
        }

        // Parse chapter title and number
        let first_line = lines[0].trim();
        let (number, title) = if let Some(colon_pos) = first_line.find(':') {
            let (chapter_part, title_part) = first_line.split_at(colon_pos);
            let number = chapter_part
                .trim()
                .strip_prefix("Chapter ")
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            (number, title_part[1..].trim().to_string())
        } else {
            (0, first_line.to_string())
        };

        Self {
            number,
            title,
            outline: ChapterOutline::default(),
            scenes: Vec::new(),
            content: String::new(),
        }
    }
}

impl fmt::Display for Chapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Chapter {}: {}\n\n{}",
            self.number,
            self.title,
            self.scenes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n\n")
        )
    }
}
