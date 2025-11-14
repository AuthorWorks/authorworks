use crate::config::Config;
use crate::error::{Result, BookGeneratorError};
use crate::sanitize_directory_name;
use std::path::Path;
use std::fs;
use std::time::Duration;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::utils::logging::TokenTracker;
use crate::book::outline::SceneOutline;
use crate::book::characters::Character;
use crate::book::scene::Scene;

mod braindump;
mod genre;
mod style;
mod synopsis;
mod characters;
pub mod outline;
pub mod chapter;
pub mod scene;

pub use braindump::Braindump;
pub use genre::Genre;
pub use style::Style;
pub use synopsis::Synopsis;
pub use characters::Characters;
pub use self::outline::{Outline, ChapterOutline};
pub use self::chapter::Chapter;

pub mod tabula;

pub use self::tabula::TemporarySummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub title: String,
    pub braindump: Braindump,
    pub genre: Genre,
    pub style: Style,
    pub characters: Characters,
    pub synopsis: Synopsis,
    pub outline: Outline,
    pub temporary_summary: TemporarySummary,
    pub history: Vec<String>,
}

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: String,
        braindump: Braindump,
        genre: Genre,
        style: Style,
        characters: Characters,
        synopsis: Synopsis,
        outline: Outline,
        temporary_summary: TemporarySummary,
    ) -> Self {
        Self {
            title,
            braindump,
            genre,
            style,
            characters,
            synopsis,
            outline,
            temporary_summary,
            history: Vec::new(),
        }
    }

    pub fn add_to_history(&mut self, entry: String) {
        self.history.push(entry);
    }

    pub fn get_full_context(&self) -> String {
        self.format_context(true)
    }

    pub fn get_aggregated_context(&self) -> String {
        self.format_context(false)
    }
    
    fn format_context(&self, outline_first: bool) -> String {
        let common_prefix = format!(
            "Title: {}\nBraindump: {}\nGenre: {} - {}\nStyle: {}\nSynopsis: {}",
            self.title,
            self.braindump.content,
            self.genre.name,
            self.genre.description,
            self.style.description,
            self.synopsis.content,
        );
        
        let (part1, part2) = if outline_first {
            (format!("\nOutline: {}", self.outline), format!("\nCharacters: {}", self.characters))
        } else {
            (format!("\nCharacters: {}", self.characters), format!("\nOutline: {}", self.outline))
        };
        
        format!(
            "{}{}{}\nTemporary Synopsis: {}",
            common_prefix,
            part1,
            part2,
            self.temporary_summary.content
        )
    }
}

#[derive(Debug)]
pub struct Book {
    pub title: String,
    pub context: Context,
    pub chapters: Vec<Chapter>,
    pub generation_time: Option<Duration>,
    pub phase_timings: Option<HashMap<String, Duration>>,
    pub is_completed: bool,
}

impl Book {
    pub async fn generate(config: &Config) -> Result<Self> {
        let title = String::new();
        let output_dir = Path::new("output").join(sanitize_directory_name(&title));
        fs::create_dir_all(&output_dir)?;
        
        // Create a token tracker to monitor usage throughout the generation process
        let token_tracker = TokenTracker::new();

        let mut context = Context::new(
            title.clone(),
            Braindump::default(),
            Genre::default(),
            Style::default(),
            Characters::default(),
            Synopsis::default(),
            Outline::default(),
            TemporarySummary::default(),
        );

        context.braindump = Braindump::generate_or_input((None, &output_dir), &mut context, config, &token_tracker).await?;
        context.genre = Genre::generate_or_input((title.clone(), &output_dir), &mut context, config, &token_tracker).await?;
        context.style = Style::generate_or_input((title.clone(), &output_dir), &mut context, config, &token_tracker).await?;
        context.characters = Characters::generate_or_input((title.clone(), &output_dir), &mut context, config, &token_tracker).await?;
        context.synopsis = Synopsis::generate_or_input((title.clone(), &output_dir), &mut context, config, &token_tracker).await?;
        context.outline = Outline::generate_or_input((title.clone(), &output_dir), &mut context, config, &token_tracker).await?;

        let chapters = generate_chapters(&mut context, config, &output_dir, &token_tracker).await?;

        Ok(Self { 
            title, 
            context, 
            chapters,
            generation_time: None,
            phase_timings: None,
            is_completed: false,
        })
    }

    // When loading a book from an existing directory, use the safer metadata reading functions
    pub fn load_from_directory(output_dir: &Path) -> Result<Self> {
        // Load metadata components
        let title = if let Some(title) = crate::read_metadata_section_safe(output_dir, "title") {
            title
        } else {
            output_dir.file_name().unwrap().to_string_lossy().to_string()
        };
        
        println!("Loading book '{}' from directory: {:?}", title, output_dir);
        
        // Safely load other metadata components
        let braindump = if let Some(content) = crate::read_metadata_section_safe(output_dir, "braindump") {
            Braindump { content }
        } else {
            Braindump::default()
        };
        
        let genre = if let Some(content) = crate::read_metadata_section_safe(output_dir, "genre") {
            // Try to parse as name: description format
            let parts: Vec<&str> = content.splitn(2, ':').collect();
            if parts.len() == 2 {
                Genre {
                    name: parts[0].trim().to_string(),
                    description: parts[1].trim().to_string(),
                }
            } else {
                Genre {
                    name: "Unknown".to_string(),
                    description: content,
                }
            }
        } else {
            Genre::default()
        };
        
        let style = if let Some(content) = crate::read_metadata_section_safe(output_dir, "style") {
            Style { description: content }
        } else {
            Style::default()
        };
        
        let characters = if let Some(content) = crate::read_metadata_section_safe(output_dir, "characters") {
            if let Ok(parsed) = serde_json::from_str::<Characters>(&content) {
                parsed
            } else {
                // Simple parsing fallback
                let mut list = Vec::new();
                for line in content.lines() {
                    if !line.trim().is_empty() {
                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            list.push(Character {
                                name: parts[0].trim().to_string(),
                                description: parts[1].trim().to_string(),
                            });
                        }
                    }
                }
                Characters { list }
            }
        } else {
            Characters::default()
        };
        
        let synopsis = if let Some(content) = crate::read_metadata_section_safe(output_dir, "synopsis") {
            Synopsis { content }
        } else {
            Synopsis::default()
        };
        
        // Load outline
        let outline = if let Some(content) = crate::read_metadata_section_safe(output_dir, "outline") {
            if let Ok(parsed) = serde_json::from_str::<Outline>(&content) {
                parsed
            } else {
                // Try to load from outline.json if metadata parsing fails
                let outline_file = output_dir.join("outline.json");
                if outline_file.exists() {
                    if let Ok(outline_content) = std::fs::read_to_string(&outline_file) {
                        if let Ok(parsed_outline) = serde_json::from_str::<Outline>(&outline_content) {
                            parsed_outline
                        } else {
                            // Create a simple outline from the content
                            Outline { chapters: Vec::new() }
                        }
                    } else {
                        Outline::default()
                    }
                } else {
                    Outline::default()
                }
            }
        } else {
            // Try to load from outline.json if metadata section doesn't exist
            let outline_file = output_dir.join("outline.json");
            if outline_file.exists() {
                if let Ok(outline_content) = std::fs::read_to_string(&outline_file) {
                    if let Ok(parsed_outline) = serde_json::from_str::<Outline>(&outline_content) {
                        parsed_outline
                    } else {
                        // If we can't parse the outline, we can't proceed
                        println!("Failed to parse outline.json");
                        
                        // Create a context with what we have and return an empty book
                        let context = Context::new(
                            title.clone(),
                            braindump.clone(),
                            genre.clone(),
                            style.clone(),
                            characters.clone(),
                            synopsis.clone(),
                            Outline::default(),
                            TemporarySummary::default(),
                        );
                        
                        return Ok(Self { 
                            title: title.clone(), 
                            context, 
                            chapters: Vec::new(),
                            generation_time: None,
                            phase_timings: None,
                            is_completed: false,
                        });
                    }
                } else {
                    Outline::default()
                }
            } else {
                Outline::default()
            }
        };
        
        // Create context with loaded components
        let context = Context::new(
            title.clone(),
            braindump,
            genre,
            style,
            characters,
            synopsis,
            outline.clone(),
            TemporarySummary::default(),
        );
        
        // Load existing chapters
        let logs_dir = output_dir.join("logs");
        let mut chapters = Vec::new();
        
        if logs_dir.exists() {
            println!("Scanning logs directory for existing chapters...");
            
            // First, check for chapter_*.md files
            let mut chapter_files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let file_name = path.file_name().unwrap().to_string_lossy();
                        if file_name.starts_with("chapter_") && file_name.ends_with(".md") && !file_name.contains("scene") {
                            chapter_files.push(path);
                        }
                    }
                }
            }
            
            // If no chapter_*.md files found, look for chapter_generation_* files
            if chapter_files.is_empty() {
                println!("No chapter_*.md files found, looking for chapter generation files...");
                if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            let file_name = path.file_name().unwrap().to_string_lossy();
                            if file_name.starts_with("chapter_generation_") && 
                               file_name.ends_with(".txt") && 
                               !file_name.contains("token_usage") {
                                chapter_files.push(path);
                            }
                        }
                    }
                }
            }
            
            // If still no files found, look for temporary summary files
            if chapter_files.is_empty() {
                println!("No chapter generation files found, looking for temporary summary files...");
                if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            let file_name = path.file_name().unwrap().to_string_lossy();
                            if file_name.starts_with("temporary_summary_chapter_") && 
                               !file_name.contains("token_usage") {
                                chapter_files.push(path);
                            }
                        }
                    }
                }
            }
            
            // Sort chapter files by chapter number
            chapter_files.sort_by(|a, b| {
                let a_file = a.file_name().unwrap().to_string_lossy();
                let b_file = b.file_name().unwrap().to_string_lossy();
                
                let a_num = if a_file.starts_with("chapter_") && a_file.ends_with(".md") {
                    a_file.replace("chapter_", "").replace(".md", "")
                        .parse::<usize>().unwrap_or(0)
                } else if a_file.starts_with("chapter_generation_") {
                    a_file.strip_prefix("chapter_generation_")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0)
                } else if a_file.starts_with("temporary_summary_chapter_") {
                    a_file.strip_prefix("temporary_summary_chapter_")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0)
                } else {
                    0
                };
                
                let b_num = if b_file.starts_with("chapter_") && b_file.ends_with(".md") {
                    b_file.replace("chapter_", "").replace(".md", "")
                        .parse::<usize>().unwrap_or(0)
                } else if b_file.starts_with("chapter_generation_") {
                    b_file.strip_prefix("chapter_generation_")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0)
                } else if b_file.starts_with("temporary_summary_chapter_") {
                    b_file.strip_prefix("temporary_summary_chapter_")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0)
                } else {
                    0
                };
                
                a_num.cmp(&b_num)
            });
            
            println!("Found {} potential chapter files", chapter_files.len());
            
            // Track chapters we've already processed to avoid duplicates
            let mut processed_chapter_numbers = std::collections::HashSet::new();
            
            // Load chapters
            for chapter_file in chapter_files {
                if let Ok(content) = std::fs::read_to_string(&chapter_file) {
                    let file_name = chapter_file.file_name().unwrap().to_string_lossy();
                    
                    // Extract chapter number from filename
                    let chapter_num = if file_name.starts_with("chapter_") && file_name.ends_with(".md") {
                        file_name.replace("chapter_", "").replace(".md", "")
                            .parse::<usize>().unwrap_or(0)
                    } else if file_name.starts_with("chapter_generation_") {
                        file_name.strip_prefix("chapter_generation_")
                            .and_then(|s| s.split('_').next())
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(0)
                    } else if file_name.starts_with("temporary_summary_chapter_") {
                        file_name.strip_prefix("temporary_summary_chapter_")
                            .and_then(|s| s.split('_').next())
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    
                    // Skip if we've already processed this chapter number or if chapter number is invalid
                    if chapter_num == 0 || processed_chapter_numbers.contains(&chapter_num) {
                        continue;
                    }
                    
                    println!("Loading chapter {} from file: {}", chapter_num, file_name);
                    
                    // Try to parse the chapter from JSON
                    let mut chapter = if let Ok(parsed_chapter) = serde_json::from_str::<Chapter>(&content) {
                        parsed_chapter
                    } else {
                        // Manual parsing fallback
                        println!("  Could not parse chapter {} as JSON, using fallback method", chapter_num);
                        
                        // Try to load the outline from the book outline
                        let chapter_outline = if let Some(outline_chapter) = outline.chapters.get(chapter_num - 1).cloned() {
                            println!("  Found outline for chapter {} in book outline", chapter_num);
                            outline_chapter
                        } else {
                            // Try to parse the chapter outline from the content
                            println!("  Attempting to extract chapter outline from content for chapter {}", chapter_num);
                            
                            // Extract title from the content
                            let title = content.lines()
                                .find(|line| line.contains("Chapter") && line.contains(":"))
                                .map(|line| {
                                    line.split(":")
                                        .nth(1)
                                        .unwrap_or("Untitled Chapter")
                                        .trim()
                                        .to_string()
                                })
                                .unwrap_or_else(|| format!("Chapter {}", chapter_num));
                            
                            println!("  Extracted title for chapter {}: {}", chapter_num, title);
                            
                            // Create a basic chapter outline with empty scenes
                            let mut chapter_outline = crate::book::outline::ChapterOutline {
                                chapter_number: chapter_num,
                                title: title.clone(),
                                description: content.lines().take(10).collect::<Vec<_>>().join("\n"),
                                scenes: Vec::new(),
                            };
                            
                            // Try to extract scene outlines from the content
                            let mut scene_outlines = Vec::new();
                            let mut current_scene_title = String::new();
                            let mut current_scene_description = Vec::new();
                            let mut in_scene = false;
                            let mut scene_number = 0;
                            
                            // Limit the number of scenes to extract to avoid unrealistic numbers
                            let max_scenes_per_chapter = 10;
                            
                            for line in content.lines() {
                                if line.contains("Scene") && line.contains(":") {
                                    // If we were already in a scene, save it
                                    if in_scene && !current_scene_title.is_empty() {
                                        scene_outlines.push(crate::book::outline::SceneOutline {
                                            number: scene_number,
                                            title: current_scene_title.clone(),
                                            description: current_scene_description.join("\n"),
                                        });
                                    }
                                    
                                    // Start a new scene
                                    scene_number += 1;
                                    
                                    // Stop if we've reached the maximum number of scenes
                                    if scene_number > max_scenes_per_chapter {
                                        break;
                                    }
                                    
                                    in_scene = true;
                                    current_scene_title = line.split(":")
                                        .nth(1)
                                        .unwrap_or("Untitled Scene")
                                        .trim()
                                        .to_string();
                                    current_scene_description = Vec::new();
                                } else if in_scene {
                                    // Add line to current scene description
                                    current_scene_description.push(line.to_string());
                                }
                            }
                            
                            // Add the last scene if there is one and we haven't reached the maximum
                            if in_scene && !current_scene_title.is_empty() && scene_number <= max_scenes_per_chapter {
                                scene_outlines.push(crate::book::outline::SceneOutline {
                                    number: scene_number,
                                    title: current_scene_title,
                                    description: current_scene_description.join("\n"),
                                });
                            }
                            
                            // Add extracted scenes to the chapter outline
                            if !scene_outlines.is_empty() {
                                println!("  Extracted {} scene outlines for chapter {}", scene_outlines.len(), chapter_num);
                                chapter_outline.scenes = scene_outlines;
                            }
                            
                            chapter_outline
                        };
                        
                        // Create the chapter
                        Chapter {
                            number: chapter_num,
                            title: chapter_outline.title.clone(),
                            outline: chapter_outline,
                            scenes: Vec::new(),
                            content,
                        }
                    };
                    
                    // Load scenes for this chapter if they exist
                    let scene_pattern = format!("scene_generation_ch{}_scene", chapter_num);
                    let content_pattern = format!("content_generation_ch{}_scene", chapter_num);
                    let generic_scene_pattern = "scene_generation_ch";
                    let mut scene_files = Vec::new();
                    
                    // Collect scene files
                    if let Ok(scene_entries) = std::fs::read_dir(&logs_dir) {
                        for entry in scene_entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                let file_name = path.file_name().unwrap().to_string_lossy();
                                
                                // Check if the file matches our patterns
                                let is_scene_file = file_name.contains(&scene_pattern) || 
                                                   file_name.contains(&content_pattern) ||
                                                   (file_name.contains(generic_scene_pattern) && !file_name.contains("token_usage"));
                                
                                if is_scene_file && 
                                   !file_name.ends_with("_token_usage.txt") &&
                                   !file_name.contains("token_usage") {
                                    
                                    // Extract chapter number from filename to verify it's for this chapter
                                    let file_chapter_num = if file_name.contains("ch") {
                                        file_name.split("ch").nth(1)
                                            .and_then(|s| s.split('_').next())
                                            .and_then(|s| s.parse::<usize>().ok())
                                    } else {
                                        None
                                    };
                                    
                                    // Only add if this file is for the current chapter or has no specific chapter
                                    if file_chapter_num.is_none() || file_chapter_num == Some(chapter_num) {
                                        scene_files.push(path);
                                    }
                                }
                            }
                        }
                    }
                    
                    // Sort scene files by scene number
                    scene_files.sort_by(|a, b| {
                        let a_file = a.file_name().unwrap().to_str().unwrap();
                        let b_file = b.file_name().unwrap().to_str().unwrap();
                        
                        let a_num = if a_file.contains("scene") {
                            a_file.split("scene").nth(1)
                                .and_then(|s| s.split('_').next())
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        
                        let b_num = if b_file.contains("scene") {
                            b_file.split("scene").nth(1)
                                .and_then(|s| s.split('_').next())
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        
                        a_num.cmp(&b_num)
                    });
                    
                    println!("  Found {} scene files for chapter {}", scene_files.len(), chapter_num);
                    
                    // Load scenes
                    let mut scenes = Vec::new();
                    for scene_file in scene_files {
                        if let Ok(scene_content) = std::fs::read_to_string(&scene_file) {
                            let file_name = scene_file.file_name().unwrap().to_str().unwrap();
                            
                            // Extract scene number from filename
                            let scene_num = if file_name.contains("scene") {
                                file_name.split("scene").nth(1)
                                    .and_then(|s| s.split('_').next())
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or(0)
                            } else {
                                0
                            };
                            
                            // If we couldn't extract a scene number, try to infer it from the scenes we've already loaded
                            let scene_num = if scene_num == 0 {
                                scenes.len() + 1
                            } else {
                                scene_num
                            };
                            
                            if scene_num > 0 && scene_num <= chapter.outline.scenes.len() {
                                let scene_outline = &chapter.outline.scenes[scene_num - 1];
                                
                                // Create a scene with the parsed outline
                                let scene = Scene {
                                    title: scene_outline.title.clone(),
                                    outline: scene_outline.clone(),
                                    content: if file_name.contains("content_generation") {
                                        // If this is a content file, use its content
                                        crate::book::content::Content { 
                                            text: scene_content.trim().to_string(),
                                            chapter_number: chapter_num,
                                            scene_number: scene_num
                                        }
                                    } else {
                                        crate::book::content::Content::default()
                                    },
                                };
                                
                                scenes.push(scene);
                                println!("    Loaded scene {} for chapter {}", scene_num, chapter_num);
                            } else if scene_num > 0 {
                                println!("    Warning: Scene {} exceeds the number of scenes in chapter outline ({})", 
                                         scene_num, chapter.outline.scenes.len());
                            }
                        }
                    }
                    
                    // Add scenes to the chapter
                    if !scenes.is_empty() {
                        println!("  Added {} scenes to chapter {}", scenes.len(), chapter_num);
                        chapter.scenes = scenes;
                    }
                    
                    chapters.push(chapter);
                    processed_chapter_numbers.insert(chapter_num);
                }
            }
            
            // Sort chapters by chapter number to ensure they're in the correct order
            chapters.sort_by_key(|chapter| chapter.number);
            
            println!("Successfully loaded {} chapters from existing directory", chapters.len());
            
            // Log the number of chapters and their scene outlines
            for chapter in &chapters {
                println!("Chapter {}: \"{}\" has {} scene outlines", 
                         chapter.number, chapter.title, chapter.outline.scenes.len());
            }
            
            Ok(Self {
                title,
                context,
                chapters,
                generation_time: None,
                phase_timings: None,
                is_completed: false,
            })
        } else {
            Err(BookGeneratorError::MissingContext("Logs directory not found".to_string()))
        }
    }

    pub fn new(title: String, context: Context, chapters: Vec<Chapter>) -> Self {
        Self {
            title,
            context,
            chapters,
            generation_time: None,
            phase_timings: None,
            is_completed: false,
        }
    }
    
    /// Add a chapter to the book
    pub fn add_chapter(&mut self, chapter: Chapter) {
        // Check if we already have this chapter
        if let Some(existing_chapter) = self.chapters.iter_mut().find(|c| c.number == chapter.number) {
            // Replace the existing chapter
            *existing_chapter = chapter;
        } else {
            // Add the new chapter
            self.chapters.push(chapter);
            
            // Sort chapters by number
            self.chapters.sort_by_key(|c| c.number);
        }
    }
}

async fn generate_chapters(
    context: &mut Context,
    config: &Config,
    output_dir: &Path,
    token_tracker: &TokenTracker,
) -> Result<Vec<Chapter>> {
    let mut chapters: Vec<Chapter> = Vec::new();
    let chapter_outlines = context.outline.chapters.clone();
    
    for (i, chapter_outline) in chapter_outlines.iter().enumerate() {
        let previous_chapters = chapters.iter()
            .map(|ch| ch.outline.clone())
            .collect::<Vec<_>>();
        let chapter = Chapter::generate(
            i + 1,
            &chapter_outline.title,
            context,
            config,
            &previous_chapters,
            output_dir,
            token_tracker
        ).await?;
        chapters.push(chapter);
    }
    Ok(chapters)
}

pub mod content;