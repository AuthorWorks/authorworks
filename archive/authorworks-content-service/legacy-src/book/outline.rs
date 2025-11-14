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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outline {
    pub chapters: Vec<ChapterOutline>,
}

impl AsRef<Outline> for Outline {
    fn as_ref(&self) -> &Outline {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterOutline {
    pub title: String,
    pub description: String,
    pub scenes: Vec<SceneOutline>,
    pub chapter_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneOutline {
    pub title: String,
    pub description: String,
    pub number: usize,
}

impl Outline {
    pub async fn generate_or_input(
        (title, output_dir): (String, &std::path::Path),
        context: &mut Context,
        config: &Config,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        // Try to load existing outline using the safer function
        if let Some(content) = crate::read_metadata_section_safe(output_dir, "Book Outline") {
            println!("Existing book outline found. Do you want to use it? (Y/n)");
            let use_existing = crate::utils::input::get_user_input("")?.to_lowercase();
            if use_existing != "n" {
                println!("Using existing book outline from metadata.");
                let outline = Self::from_string(&content);
                
                // Validate the outline to ensure it's not corrupted
                if outline.chapters.is_empty() {
                    println!("Warning: The existing outline appears to be empty or corrupted.");
                    println!("Would you like to generate a new outline instead? (Y/n)");
                    let generate_new = crate::utils::input::get_user_input("")?.to_lowercase();
                    if generate_new != "n" {
                        println!("Generating new book outline...");
                    } else {
                        return Err(BookGeneratorError::MissingContext("Outline is empty. Please provide a valid outline.".to_string()));
                    }
                } else {
                    return Ok(outline);
                }
            }
        }

        // If auto_generate is true, skip manual input prompt
        let result = if config.auto_generate {
            info!("Auto-generating book outline...");
            println!("Sending outline generation prompt to LLM");
            let base_context = format!(
                "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nCharacters: {}\nSynopsis: {}",
                title,
                context.braindump.content,
                context.genre.name,
                context.genre.description,
                context.style.description,
                context.characters,
                context.synopsis.content,
            );
            let prompt = Prompts::outline();
            
            // Log the prompt before sending
            log_prompt(
                output_dir,
                "outline_generation_prompt",
                &prompt.template(),
                &base_context,
            )?;
            
            let result = Self::ai_generate(&title, context, config, output_dir, token_tracker).await?;
            
            info!("Received book outline from LLM");
            
            let outline = Self::from_string(&result.to_string());
            
            // Store the final outline in metadata
            crate::update_metadata(
                output_dir,
                "Book Outline",
                &outline.to_string()
            )?;
            
            info!("Raw LLM output: {}", result.to_string());
            result
        } else {
            println!("Do you want to input the outline manually? (Y/n)");
            let manual_input = crate::utils::input::get_user_input("")?.to_lowercase();
            
            if manual_input != "n" {
                Self::manual_input()?
            } else {
                println!("Sending outline generation prompt to LLM");
                let base_context = format!(
                    "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nCharacters: {}\nSynopsis: {}",
                    title,
                    context.braindump.content,
                    context.genre.name,
                    context.genre.description,
                    context.style.description,
                    context.characters,
                    context.synopsis.content,
                );
                let prompt = Prompts::outline();
                
                // Log the prompt before sending
                log_prompt(
                    output_dir,
                    "outline_generation_prompt",
                    &prompt.template(),
                    &base_context,
                )?;
                
                let result = Self::ai_generate(&title, context, config, output_dir, token_tracker).await?;
                
                info!("Received book outline from LLM");
                
                let outline = Self::from_string(&result.to_string());
                
                // Store the final outline in metadata
                crate::update_metadata(
                    output_dir,
                    "Book Outline",
                    &outline.to_string()
                )?;
                
                info!("Raw LLM output: {}", result.to_string());
                result
            }
        };

        if result.chapters.is_empty() {
            return Err(BookGeneratorError::MissingContext("Outline is empty. Please provide a valid outline.".to_string()));
        }

        // Store the final outline in metadata
        context.add_to_history(format!("Outline:\n{}", result));
        
        // Store chapter count for validation
        crate::update_metadata(
            output_dir,
            "Chapter Count",
            &format!("{}", result.chapters.len())
        )?;

        Ok(result)
    }

    async fn ai_generate(title: &str, context: &Context, config: &Config, output_dir: &Path, token_tracker: &crate::utils::logging::TokenTracker) -> Result<Self> {
        let llm = crate::llm::create_llm(config)?;
        let prompt = Prompts::outline();
        
        // Log the prompt we're sending
        tracing::debug!("Sending outline generation prompt to LLM with template: {}", prompt.template());
        
        let chain = LLMChainBuilder::new()
            .prompt(prompt.clone())
            .llm(llm)
            .build()?;

        let output = chain.call(langchain_rust::prompt_args!{
            "title" => title,
            "braindump" => &context.braindump.content,
            "genre" => &format!("{} - {}", context.genre.name, context.genre.description),
            "style" => &context.style.description,
            "characters" => &context.characters.to_string(),
            "synopsis" => &context.synopsis.content
        }).await?;

        crate::log_with_tracker(
            output_dir,
            "outline_generation",
            &output.generation,
            output.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            output.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            token_tracker,
        )?;

        // Log the raw output for debugging with more detail
        tracing::debug!("Raw outline generation output (length: {}): {}", output.generation.len(), output.generation);
        tracing::debug!("First 500 characters: {}", &output.generation.chars().take(500).collect::<String>());
        
        // Save the raw output to a file for inspection
        let raw_output_path = output_dir.join("raw_outline_output.txt");
        std::fs::write(&raw_output_path, &output.generation)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to write raw outline output: {}", e)))?;
        tracing::debug!("Saved raw outline output to {:?}", raw_output_path);

        // Parse and validate the outline
        let outline = Self::parse_from_llm(&output.generation);
        
        // Log the parsed chapters for debugging
        tracing::debug!("Parsed {} chapters from outline", outline.chapters.len());
        for (i, chapter) in outline.chapters.iter().enumerate() {
            tracing::debug!("Chapter {}: {} with {} scenes", i+1, chapter.title, chapter.scenes.len());
            for (j, scene) in chapter.scenes.iter().enumerate() {
                tracing::debug!("  Scene {}: {}", j+1, scene.title);
            }
        }
        
        if outline.chapters.is_empty() {
            tracing::error!("Generated outline has no chapters. Check the parsing logic or LLM output format.");
            return Err(BookGeneratorError::Generation("Generated outline has no chapters".into()));
        }

        Ok(outline)
    }

    fn manual_input() -> Result<Self> {
        println!("Enter the outline (Type 'END' on a new line to finish):");
        let content = get_multiline_input("")?;
        Ok(Self::from_string(&content))
    }

    pub fn from_string(content: &str) -> Self {
        let mut outline = Outline { chapters: Vec::new() };
        let mut current_chapter: Option<ChapterOutline> = None;
        let mut current_scene: Option<SceneOutline> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Chapter") || trimmed.starts_with("Introduction") || trimmed.starts_with("Conclusion") {
                if let Some(mut chapter) = current_chapter.take() {  // Change: Added 'mut' here
                    if let Some(scene) = current_scene.take() {
                        chapter.scenes.push(scene);
                    }
                    outline.chapters.push(chapter);
                }
                current_chapter = Some(ChapterOutline {
                    title: trimmed.to_string(),
                    description: String::new(),
                    scenes: Vec::new(),
                    chapter_number: 0,
                });
            } else if !trimmed.is_empty() {
                if let Some(ref mut chapter) = current_chapter {
                    if chapter.description.is_empty() {
                        chapter.description = trimmed.to_string();
                    } else if current_scene.is_none() {
                        current_scene = Some(SceneOutline {
                            title: trimmed.to_string(),
                            description: String::new(),
                            number: 0,
                        });
                    } else if let Some(ref mut scene) = current_scene {
                        if scene.description.is_empty() {
                            scene.description = trimmed.to_string();
                        } else {
                            chapter.scenes.push(scene.clone());
                            current_scene = Some(SceneOutline {
                                title: trimmed.to_string(),
                                description: String::new(),
                                number: 0,
                            });
                        }
                    }
                }
            }
        }

        if let Some(mut chapter) = current_chapter.take() {  // Change: Added 'mut' here
            if let Some(scene) = current_scene.take() {
                chapter.scenes.push(scene);
            }
            outline.chapters.push(chapter);
        }

        outline
    }

    fn parse_from_llm(content: &str) -> Self {
        let mut outline = Outline { chapters: Vec::new() };
        let mut current_chapter: Option<ChapterOutline> = None;
        let mut current_scene: Option<SceneOutline> = None;

        // Log the raw content for debugging
        tracing::debug!("Parsing outline from content with {} lines", content.lines().count());
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }
            
            // Very flexible chapter title recognition with debug logging
            let is_chapter = trimmed.starts_with("Chapter ") || 
                             trimmed.starts_with("CHAPTER ") ||
                             trimmed == "Introduction" || 
                             trimmed == "Conclusion" || 
                             trimmed.starts_with("Prologue") || 
                             trimmed.starts_with("Epilogue") ||
                             (trimmed.contains("Chapter") && trimmed.contains(":"));
            
            if is_chapter {
                tracing::debug!("Found chapter title: {}", trimmed);
                
                if let Some(mut chapter) = current_chapter.take() {
                    if let Some(scene) = current_scene.take() {
                        chapter.scenes.push(scene);
                    }
                    
                    // Set chapter number if not already set
                    if chapter.chapter_number == 0 {
                        chapter.chapter_number = outline.chapters.len() + 1;
                    }
                    
                    outline.chapters.push(chapter);
                }
                
                // Extract chapter number from title if possible
                let chapter_number = if trimmed.starts_with("Chapter ") {
                    trimmed.strip_prefix("Chapter ")
                        .and_then(|s| s.split(':').next())
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(outline.chapters.len() + 1)
                } else {
                    outline.chapters.len() + 1
                };
                
                current_chapter = Some(ChapterOutline {
                    title: trimmed.to_string(),
                    description: String::new(),
                    scenes: Vec::new(),
                    chapter_number,
                });
            } else if !trimmed.is_empty() {
                if let Some(ref mut chapter) = current_chapter {
                    // Check if this line might be a scene title or description
                    let is_scene = trimmed.starts_with("Scene ") || 
                                  (trimmed.contains("Scene") && trimmed.contains(":"));
                    
                    if chapter.description.is_empty() {
                        tracing::debug!("Adding chapter description: {}", trimmed);
                        chapter.description = trimmed.to_string();
                    } else if is_scene {
                        tracing::debug!("Found scene title: {}", trimmed);
                        if let Some(scene) = current_scene.take() {
                            chapter.scenes.push(scene);
                        }
                        
                        // Extract scene number if possible
                        let scene_number = if trimmed.starts_with("Scene ") {
                            trimmed.strip_prefix("Scene ")
                                .and_then(|s| s.split(':').next())
                                .and_then(|s| s.split_whitespace().next())
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(chapter.scenes.len() + 1)
                        } else {
                            chapter.scenes.len() + 1
                        };
                        
                        current_scene = Some(SceneOutline {
                            title: trimmed.to_string(),
                            description: String::new(),
                            number: scene_number,
                        });
                    } else if current_scene.is_none() {
                        // If no current scene but we have text, create a new scene
                        current_scene = Some(SceneOutline {
                            title: format!("Scene {}", chapter.scenes.len() + 1),
                            description: trimmed.to_string(),
                            number: chapter.scenes.len() + 1,
                        });
                    } else if let Some(ref mut scene) = current_scene {
                        if scene.description.is_empty() {
                            scene.description = trimmed.to_string();
                        } else {
                            // If we already have a description, this might be a new scene
                            // without explicit "Scene" marker
                            chapter.scenes.push(scene.clone());
                            current_scene = Some(SceneOutline {
                                title: format!("Scene {}", chapter.scenes.len() + 1),
                                description: trimmed.to_string(),
                                number: chapter.scenes.len() + 1,
                            });
                        }
                    }
                } else {
                    // If we encounter text before any chapter is defined, create a default chapter
                    tracing::debug!("Creating default chapter with first line: {}", trimmed);
                    current_chapter = Some(ChapterOutline {
                        title: "Chapter 1: Introduction".to_string(),
                        description: trimmed.to_string(),
                        scenes: Vec::new(),
                        chapter_number: 1,
                    });
                }
            }
        }

        if let Some(mut chapter) = current_chapter.take() {
            if let Some(scene) = current_scene.take() {
                chapter.scenes.push(scene);
            }
            
            // Set chapter number if not already set
            if chapter.chapter_number == 0 {
                chapter.chapter_number = outline.chapters.len() + 1;
            }
            
            outline.chapters.push(chapter);
        }

        // Final debug log
        tracing::debug!("Finished parsing outline, found {} chapters", outline.chapters.len());
        
        // Ensure all chapters have proper numbers
        for (i, chapter) in outline.chapters.iter_mut().enumerate() {
            if chapter.chapter_number == 0 {
                chapter.chapter_number = i + 1;
            }
            
            // Ensure all scenes have proper numbers
            for (j, scene) in chapter.scenes.iter_mut().enumerate() {
                if scene.number == 0 {
                    scene.number = j + 1;
                }
            }
        }
        
        outline
    }
}

impl std::fmt::Display for Outline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chapter in &self.chapters {
            writeln!(f, "{}", chapter.title)?;
            writeln!(f, "{}", chapter.description)?;
            for scene in &chapter.scenes {
                writeln!(f, "{}", scene.title)?;
                writeln!(f, "{}", scene.description)?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for ChapterOutline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.title, self.description)
    }
}

impl std::fmt::Display for SceneOutline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {}: {}\n{}", self.number, self.title, self.description)
    }
}

impl ChapterOutline {
    pub fn parse_from_llm(content: &str) -> Self {
        let mut title = String::new();
        let mut description = String::new();
        let mut scenes: Vec<SceneOutline> = Vec::new();
        let mut scene_number = 0;
        
        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = None;
        let mut in_scene_description = false;
        let mut current_scene_description = String::new();
        
        for line in lines {
            let line = line.trim();
            if line.is_empty() { continue; }
            
            // Check for chapter title first
            if title.is_empty() {
                title = line.to_string();
                continue;
            }
            
            // Check for chapter description marker
            if line.starts_with("Chapter Description:") {
                if let Some(stripped) = line.strip_prefix("Chapter Description:") {
                    description = stripped.trim().to_string();
                }
                current_section = None;
                in_scene_description = false;
                continue;
            }
            
            // Check for various scene formats
            let is_scene_title = line.starts_with("Scene ") || 
                               (line.contains("Scene") && line.contains(":")) ||
                               (line.contains("Scene") && line.contains("-")) ||
                               (line.contains("Scene") && line.contains("."));
            
            if is_scene_title {
                // If we were in a scene description, save it to the previous scene
                if in_scene_description && !current_scene_description.is_empty() && !scenes.is_empty() {
                    let last_index = scenes.len() - 1;
                    scenes[last_index].description = current_scene_description.trim().to_string();
                    current_scene_description = String::new();
                }
                
                // Extract scene number and title
                scene_number += 1;
                let scene_title = if let Some(colon_pos) = line.find(':') {
                    line[colon_pos+1..].trim().to_string()
                } else if let Some(dash_pos) = line.find('-') {
                    line[dash_pos+1..].trim().to_string()
                } else if let Some(dot_pos) = line.find('.') {
                    line[dot_pos+1..].trim().to_string()
                } else {
                    line.to_string()
                };
                
                scenes.push(SceneOutline {
                    title: scene_title,
                    description: String::new(),
                    number: scene_number,
                });
                
                current_section = Some(scenes.len() - 1);
                in_scene_description = true;
                continue;
            }
            
            // Check for scene description marker
            if line.starts_with("Scene Description:") {
                if let Some(idx) = current_section {
                    if let Some(stripped) = line.strip_prefix("Scene Description:") {
                        scenes[idx].description = stripped.trim().to_string();
                        in_scene_description = false;
                    }
                }
                continue;
            }
            
            // If we're in a scene description, add this line to it
            if in_scene_description && current_section.is_some() {
                let idx = current_section.unwrap();
                if scenes[idx].description.is_empty() {
                    scenes[idx].description = line.to_string();
                } else {
                    current_scene_description.push('\n');
                    current_scene_description.push_str(line);
                }
                continue;
            }
            
            // If we haven't set a description yet, use this line
            if description.is_empty() {
                description = line.to_string();
                continue;
            }
        }
        
        // Save any pending scene description
        if in_scene_description && !current_scene_description.is_empty() && !scenes.is_empty() {
            let last_index = scenes.len() - 1;
            scenes[last_index].description = current_scene_description.trim().to_string();
        }
        
        // If no scenes were found but we have a description, try to extract scenes from the description
        if scenes.is_empty() && !description.is_empty() {
            tracing::warn!("No scenes found in chapter outline, attempting to extract from description");
            
            // Look for scene markers in the description
            let desc_lines = description.lines().collect::<Vec<&str>>();
            let mut current_scene_title = String::new();
            let mut current_scene_desc = String::new();
            scene_number = 0;
            
            for line in desc_lines {
                let line = line.trim();
                if line.is_empty() { continue; }
                
                let is_scene_marker = line.starts_with("Scene ") || 
                                    (line.contains("Scene") && line.contains(":")) ||
                                    (line.contains("Scene") && line.contains("-")) ||
                                    (line.contains("Scene") && line.contains("."));
                
                if is_scene_marker {
                    // Save previous scene if we have one
                    if !current_scene_title.is_empty() {
                        scenes.push(SceneOutline {
                            title: current_scene_title,
                            description: current_scene_desc,
                            number: scene_number,
                        });
                        current_scene_desc = String::new();
                    }
                    
                    // Extract new scene title
                    scene_number += 1;
                    current_scene_title = if let Some(colon_pos) = line.find(':') {
                        line[colon_pos+1..].trim().to_string()
                    } else if let Some(dash_pos) = line.find('-') {
                        line[dash_pos+1..].trim().to_string()
                    } else if let Some(dot_pos) = line.find('.') {
                        line[dot_pos+1..].trim().to_string()
                    } else {
                        line.to_string()
                    };
                } else if !current_scene_title.is_empty() {
                    // Add to current scene description
                    if !current_scene_desc.is_empty() {
                        current_scene_desc.push('\n');
                    }
                    current_scene_desc.push_str(line);
                }
            }
            
            // Add the last scene if we have one
            if !current_scene_title.is_empty() {
                scenes.push(SceneOutline {
                    title: current_scene_title,
                    description: current_scene_desc,
                    number: scene_number,
                });
            }
        }
        
        Self {
            title,
            description,
            scenes,
            chapter_number: 0,
        }
    }
}