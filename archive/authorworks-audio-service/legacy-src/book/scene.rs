use crate::book::{Context, ChapterOutline, SceneOutline};
use crate::book::content::Content;
use crate::config::Config;
use crate::error::Result;
use crate::TemporarySummary;
use serde::{Serialize, Deserialize};
use langchain_rust::chain::{LLMChainBuilder, Chain};
use crate::utils::prompts::Prompts;
use langchain_rust::prompt::PromptFromatter;
use tracing::info;
use std::fmt;
use std::path::Path;
use crate::book::Chapter;
use crate::error::BookGeneratorError;
use crate::utils::logging::TokenTracker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub title: String,
    pub outline: SceneOutline,
    pub content: Content,
}

impl Scene {
    pub async fn generate(
        context: &Context,
        config: &Config,
        chapter_outline: &ChapterOutline,
        scene_index: usize,
        previous_scenes: &[Scene],
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        // Check if we already have this scene in the logs directory
        let logs_dir = output_dir.join("logs");
        if logs_dir.exists() {
            let scene_pattern = format!("scene_generation_ch{}_scene{}", chapter_outline.chapter_number, scene_index + 1);
            let generic_scene_pattern = format!("scene_generation_ch_scene{}", scene_index + 1);
            
            if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let file_name = path.file_name().unwrap().to_string_lossy();
                        
                        if (file_name.contains(&scene_pattern) || file_name.contains(&generic_scene_pattern)) && 
                           !file_name.contains("token_usage") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                println!("Found existing scene file: {}", file_name);
                                
                                // Get the scene outline
                                if scene_index < chapter_outline.scenes.len() {
                                    let scene_outline = &chapter_outline.scenes[scene_index];
                                    
                                    return Ok(Self {
                                        title: scene_outline.title.clone(),
                                        outline: scene_outline.clone(),
                                        content: crate::book::content::Content::default(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If we didn't find an existing scene, generate a new one
        // Get the scene outline from the chapter outline
        let scene_outline = if scene_index < chapter_outline.scenes.len() {
            chapter_outline.scenes[scene_index].clone()
        } else {
            return Err(crate::error::BookGeneratorError::Generation(
                format!("Scene index {} (0-based) is out of bounds for chapter with {} scenes", 
                    scene_index, 
                    chapter_outline.scenes.len()
                )
            ));
        };
        
        println!("📝 Generating temporary summary for scene {}.{}", chapter_outline.chapter_number, scene_index + 1);
        // Generate temporary summary first
        let temp_summary = TemporarySummary::generate_scene(
            &context.title,
            context,
            config,
            chapter_outline,
            previous_scenes,
            output_dir,
            token_tracker,
        ).await?;

        println!("�� Generating scene {}.{}: \"{}\"", chapter_outline.chapter_number, scene_index + 1, scene_outline.title);
        info!("Sending scene generation prompt to LLM");
        
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::scene();
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        // Format previous scenes for context
        let previous_scenes_context = if previous_scenes.is_empty() {
            "This is the first scene in the chapter.".to_string()
        } else {
            let scenes = previous_scenes.iter()
                .map(|s| format!("Scene {}: {}\n{}", 
                    s.outline.number, 
                    s.title, 
                    s.outline.description
                ))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("Previous scenes in this chapter:\n\n{}", scenes)
        };

        // Log the scene generation prompt
        crate::utils::logging::log_prompt(
            output_dir,
            &format!("scene_generation_ch{}_scene{}", chapter_outline.chapter_number, scene_index + 1),
            &format!(
                "Synopsis: {}\nChapter Outline: {}\nPrevious Scenes: {}\nTemporary Summary: {}",
                context.synopsis.content,
                chapter_outline,
                previous_scenes_context,
                temp_summary.content
            ),
            &prompt.template()
        )?;

        // Use only the required context for scene generation as specified:
        // - temporary scene summary
        // - book synopsis
        // - chapter outline
        // - previous scene outlines from the same chapter
        let output = chain.call(langchain_rust::prompt_args!{
            "synopsis" => &context.synopsis.content,
            "chapter_outline" => &chapter_outline,
            "previous_scenes" => &previous_scenes_context,
            "temporary_summary" => &temp_summary.content
        }).await?;

        crate::log_with_tracker(
            output_dir,
            &format!("scene_generation_ch{}_scene{}", chapter_outline.chapter_number, scene_index + 1),
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        println!("✅ Successfully generated scene {}.{}: \"{}\"", chapter_outline.chapter_number, scene_index + 1, scene_outline.title);

        // Create a scene with the generated outline
        let scene = Self {
            title: scene_outline.title.clone(),
            outline: scene_outline,
            content: Content::default(),
        };
        
        Ok(scene)
    }

    /// Generate content for this scene
    pub async fn generate_content(
        &self,
        context: &mut Context,
        config: &Config,
        chapter: &Chapter,
        output_dir: &Path,
        token_tracker: &TokenTracker,
    ) -> crate::error::Result<Content> {
        // Check if API is available
        if !crate::llm::api_available().await {
            return Err(BookGeneratorError::LLMError("API not available".to_string()));
        }
        
        // Generate content using the Content::generate method
        let content = Content::generate(
            context,
            config,
            chapter.number,
            self.outline.number,
            &chapter.title,
            &self.title,
            &self.outline.description,
            &chapter.outline.description,
            output_dir,
            token_tracker,
        ).await?;
        
        Ok(content)
    }
}

impl fmt::Display for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scene: {}\n\n{}", self.title, self.content.text)
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}