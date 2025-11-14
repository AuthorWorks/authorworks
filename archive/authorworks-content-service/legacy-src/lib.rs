#![allow(unused)]

mod config;
mod book;
mod chain;
pub mod utils;
mod render;
pub mod llm;
pub mod error;

use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::book::outline::ChapterOutline;
use crate::book::{Braindump, Genre, Style, Synopsis, Characters, Context};
use crate::book::tabula::TemporarySummary;
use serde::{Serialize, Deserialize};
use crate::error::{Result, BookGeneratorError};
use crate::utils::time_utils::format_duration;

pub use config::Config;
pub use book::Book;
pub use book::chapter::Chapter;
pub use book::scene::Scene;
pub use book::content::Content;
pub use book::outline::Outline;
pub use utils::statistics::BookStatistics;

// Single re-export for all render functions
pub use crate::render::{render_book, generate_pdf_and_epub};

// Add the re-export of file_utils functions
pub use crate::utils::file_utils::{
    sanitize_directory_name, 
    read_metadata_file_safe, 
    read_metadata_section_safe,
    read_metadata_section,
    update_metadata,
    update_metadata_json,
    check_for_complete_book,
    find_existing_content_file, 
    extract_scene_number,
    sanitize_filename
};

// Add re-export for time_utils functions

/// Metadata for a book
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub title: String,
    pub braindump: Braindump,
    pub genre: Genre,
    pub style: Style,
    pub characters: Characters,
    pub synopsis: Synopsis,
    pub outline: Outline,
}

pub async fn generate_book(
    book_title: String,
    config: &Config,
) -> Result<(Book, utils::logging::TokenTracker)> {
    // Sanitize the directory name from the book title
    let output_dir = PathBuf::from("output").join(sanitize_directory_name(&book_title));
    
    // Create the output directory
    fs::create_dir_all(&output_dir)?;
    
    // Generate the book with the given directory
    generate_book_with_dir(book_title, config, &output_dir, config.auto_generate).await
}

pub async fn generate_book_with_dir(
    book_title: String,
    config: &Config,
    output_dir: &Path,
    auto_generate: bool
) -> Result<(Book, utils::logging::TokenTracker)> {
    // Start timing the entire process
    let start_time = Instant::now();
    let mut phase_timings = HashMap::new();
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    // Check if we're resuming an existing book
    let resuming = output_dir.exists() && output_dir.join("metadata.md").exists();
    
    // Check if the book generation is already complete
    let completion_flag = output_dir.join("book_complete.flag");
    if completion_flag.exists() {
        println!("📚 Book '{}' is already fully generated", book_title);
        println!("Loading the completed book for rendering...");
        
        // Load the existing book
        let mut book = match Book::load_from_directory(output_dir) {
            Ok(loaded_book) => {
                println!("Successfully loaded existing book with {} chapters", loaded_book.chapters.len());
                loaded_book
            },
            Err(e) => {
                return Err(BookGeneratorError::Generation(format!("Failed to load completed book: {}", e)));
            }
        };
        
        // Set a flag on the book to indicate it was loaded from a completed state
        book.is_completed = true;
        
        // Return the book without rendering - let the caller handle the rendering
        return Ok((book, utils::logging::TokenTracker::new()));
    }
    
    if resuming {
        println!("📚 Resuming book generation for '{}'", book_title);
    } else {
        println!("📚 Starting new book generation for '{}'", book_title);
    }
    
    // Initialize token tracker
    let token_tracker = utils::logging::TokenTracker::new();
    
    // Clean up old logs if log retention is enabled
    if config.log_retention_days > 0 {
        match utils::logging::cleanup_logs(output_dir, config.log_retention_days, true) {
            Ok(count) => {
                if count > 0 {
                    println!("🧹 Cleaned up {} old log files", count);
                }
            },
            Err(e) => {
                println!("⚠️ Warning: Failed to clean up old logs: {}", e);
            }
        }
    }
    
    // Check API availability before starting - use the lightweight check
    println!("🔍 Checking Anthropic API availability before starting...");
    if !crate::utils::api_verification::wait_for_api_availability(Some(std::time::Duration::from_secs(60))).await {
        return Err(BookGeneratorError::Generation("Anthropic API is unavailable after waiting. Please try again later.".into()));
    }
    
    // Start API status monitoring in the background - this now uses the global monitor
    crate::utils::api_verification::start_api_status_monitor(Duration::from_secs(30));
    
    // Check if we're resuming from an existing book with metadata
    let resuming = output_dir.join("metadata.md").exists();
    
    // If we're resuming, try to load the existing book
    let mut book = if resuming {
        match Book::load_from_directory(output_dir) {
            Ok(mut loaded_book) => {
                println!("Successfully loaded existing book with {} chapters", loaded_book.chapters.len());
                
                // Update the title in metadata if it's missing
                if crate::read_metadata_section_safe(output_dir, "Title").is_none() {
                    println!("Adding missing title to metadata file");
                    update_metadata(output_dir, "Title", &book_title)?;
                }
                
                // Check if any chapters have empty scenes and generate them if needed
                for chapter in &mut loaded_book.chapters {
                    if chapter.scenes.is_empty() {
                        println!("Chapter {} has no scenes, generating them...", chapter.number);
                        
                        // First check if the chapter has scene outlines
                        if chapter.outline.scenes.is_empty() {
                            println!("  Chapter {} has no scene outlines in its chapter outline, skipping scene generation", chapter.number);
                            continue;
                        }
                        
                        // Generate scenes for this chapter
                        println!("  🎬 Generating scenes for chapter {} based on {} scene outlines", 
                                chapter.number, chapter.outline.scenes.len());
                        
                        // Get the chapter outline from the book outline
                        let chapter_outline_from_book = loaded_book.context.outline.chapters.iter()
                            .find(|ch| ch.chapter_number == chapter.number)
                            .cloned();
                        
                        if let Some(ch_outline) = chapter_outline_from_book {
                            if !ch_outline.scenes.is_empty() && ch_outline.scenes.len() > chapter.outline.scenes.len() {
                                println!("  📝 Using scene outlines from the book outline for chapter {} ({} scenes)", 
                                        chapter.number, ch_outline.scenes.len());
                                chapter.outline.scenes = ch_outline.scenes.clone();
                            }
                        }
                        
                        chapter.scenes = generate_scenes_for_chapter(
                            &mut loaded_book.context,
                            config,
                            chapter,
                            output_dir,
                            &token_tracker
                        ).await?;
                        
                        println!("  ✅ Generated {} scenes for chapter {}", chapter.scenes.len(), chapter.number);
                    } else {
                        println!("  Chapter {} already has {} scenes", chapter.number, chapter.scenes.len());
                    }
                }
                
                loaded_book
            },
            Err(e) => {
                println!("Warning: Failed to load existing book: {}", e);
                println!("Starting with a fresh book generation");
                
                // Initialize context with defaults
                let context = Context::new(
                    book_title.to_string(),
                    Braindump::default(),
                    Genre::default(),
                    Style::default(),
                    Characters::default(),
                    Synopsis::default(),
                    Outline::default(),
                    TemporarySummary::default(),
                );
                
                Book::new(book_title.to_string(), context, Vec::new())
            }
        }
    } else {
        // Initialize context with defaults for a new book
        let context = Context::new(
            book_title.clone(),
            Braindump::default(),
            Genre::default(),
            Style::default(),
            Characters::default(),
            Synopsis::default(),
            Outline::default(),
            TemporarySummary::default(),
        );
        
        Book::new(book_title.clone(), context, Vec::new())
    };
    
    // Ensure the book title is set correctly
    book.title = book_title.clone();
    
    // Also ensure the title in the context is set correctly
    book.context.title = book_title.clone();

    println!("🚀 Starting book generation process...\n");

    println!("📚 Phase 1: Initial Setup and Context");
    println!("----------------------------------");
    let phase1_start = Instant::now();
    
    println!("🧠 Generating braindump...");
    book.context.braindump = Braindump::generate_or_input((Some(book_title.clone()), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Braindump", &book.context.braindump.content)?;

    println!("📋 Generating genre...");
    book.context.genre = Genre::generate_or_input((book_title.clone(), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Genre", &format!("{}: {}", book.context.genre.name, book.context.genre.description))?;

    println!("✒️ Generating style...");
    book.context.style = Style::generate_or_input((book_title.clone(), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Style", &book.context.style.description)?;

    println!("👥 Generating characters...");
    book.context.characters = Characters::generate_or_input((book_title.clone(), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Characters", &book.context.characters.to_string())?;

    println!("📝 Generating synopsis...");
    book.context.synopsis = Synopsis::generate_or_input((book_title.clone(), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Synopsis", &book.context.synopsis.content)?;
    
    let phase1_duration = phase1_start.elapsed();
    phase_timings.insert("Phase 1: Initial Setup and Context".to_string(), phase1_duration);
    println!("Phase 1 completed in {}", format_duration(phase1_duration));

    println!("\n📋 Phase 2: Book Structure");
    println!("----------------------");
    let phase2_start = Instant::now();
    
    println!("📑 Generating book outline...");
    book.context.outline = Outline::generate_or_input((book_title.clone(), output_dir), &mut book.context, config, &token_tracker).await?;
    update_metadata(output_dir, "Book Outline", &book.context.outline.to_string())?;
    
    let phase2_duration = phase2_start.elapsed();
    phase_timings.insert("Phase 2: Book Structure".to_string(), phase2_duration);
    println!("Phase 2 completed in {}", format_duration(phase2_duration));

    println!("\n📖 Phase 3: Chapter Generation");
    println!("-------------------------");
    let phase3_start = Instant::now();
    
    let chapter_outlines = book.context.outline.chapters.clone();
    
    // Track chapter titles to prevent duplicates
    let mut seen_titles = std::collections::HashSet::new();
    
    // Add existing chapter titles to the seen_titles set
    for chapter in &book.chapters {
        seen_titles.insert(chapter.title.clone());
    }
    
    // Ensure we don't exceed the maximum number of chapters specified in config
    let max_chapters = std::cmp::min(chapter_outlines.len(), config.max_chapters);
    
    // Log the total number of chapters to be generated
    info!("Generating {} chapters based on the book outline (max: {})", max_chapters, config.max_chapters);
    
    // Determine the starting chapter number based on existing chapters
    let mut start_chapter_index = book.chapters.len();
    
    // Log the starting point clearly
    println!("Resuming book generation from chapter {} (0-indexed: {})", 
             start_chapter_index + 1, start_chapter_index);
    
    // Double-check logs directory for any chapters that might not have been loaded
    let logs_dir = output_dir.join("logs");
    if logs_dir.exists() {
        println!("Scanning logs directory for any missed chapters...");
        let mut found_chapters = 0;
        let mut highest_chapter_number = 0;
        
        // First, find the highest chapter number we already have
        for chapter in &book.chapters {
            highest_chapter_number = std::cmp::max(highest_chapter_number, chapter.number);
        }
        
        // Check for chapter files that might have been missed
        if let Ok(entries) = std::fs::read_dir(&logs_dir) {
            for entry in entries.filter_map(|r| r.ok()) {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        // Look for chapter generation files with various patterns
                        let is_chapter_file = 
                            (file_name_str.starts_with("chapter_") && file_name_str.ends_with(".json")) ||
                            (file_name_str.starts_with("chapter_generation_") && 
                             !file_name_str.contains("token_usage") && 
                             file_name_str.ends_with(".txt")) ||
                            (file_name_str.contains("chapter_") && file_name_str.ends_with(".md"));
                        
                        if is_chapter_file {
                            // Try to extract chapter number using various patterns
                            let chapter_num = if file_name_str.starts_with("chapter_") && file_name_str.ends_with(".json") {
                                file_name_str
                                    .strip_prefix("chapter_")
                                    .and_then(|s| s.strip_suffix(".json"))
                                    .and_then(|s| s.parse::<usize>().ok())
                            } else if file_name_str.starts_with("chapter_generation_") {
                                file_name_str
                                    .strip_prefix("chapter_generation_")
                                    .and_then(|s| s.split('_').next())
                                    .and_then(|s| s.parse::<usize>().ok())
                            } else if file_name_str.contains("chapter_") && file_name_str.ends_with(".md") {
                                file_name_str
                                    .split("chapter_")
                                    .nth(1)
                                    .and_then(|s| s.split('.').next())
                                    .and_then(|s| s.parse::<usize>().ok())
                            } else {
                                None
                            };
                            
                            if let Some(chapter_num) = chapter_num {
                                // Check if we already have this chapter
                                if !book.chapters.iter().any(|c| c.number == chapter_num) && chapter_num > highest_chapter_number {
                                    found_chapters += 1;
                                    highest_chapter_number = std::cmp::max(highest_chapter_number, chapter_num);
                                    println!("  Found potential chapter {} in logs that wasn't loaded", chapter_num);
                                }
                            }
                        }
                        
                        // Also check for outline files which might contain chapter information
                        if file_name_str.contains("outline_generation") && 
                           !file_name_str.contains("token_usage") {
                            println!("  Found outline generation file that might contain chapter information: {}", file_name_str);
                        }
                    }
                }
            }
        }
        
        // Also check for metadata file which might have chapter information
        let metadata_path = output_dir.join("metadata.md");
        if metadata_path.exists() {
            if let Ok(_content) = std::fs::read_to_string(&metadata_path) {
                if let Some(outline_section) = read_metadata_section_safe(output_dir, "Book Outline") {
                    // Count chapters in the outline
                    let chapter_count = outline_section.lines()
                        .filter(|line| line.starts_with("Chapter ") || 
                                       line.starts_with("CHAPTER ") || 
                                       *line == "Introduction" || 
                                       *line == "Conclusion")
                        .count();
                    
                    if chapter_count > highest_chapter_number {
                        println!("  Found {} chapters in the book outline, but only {} loaded", 
                                 chapter_count, highest_chapter_number);
                        highest_chapter_number = chapter_count;
                    }
                }
                
                // Check for Chapter Count metadata
                if let Some(count_str) = read_metadata_section_safe(output_dir, "Chapter Count") {
                    if let Ok(count) = count_str.trim().parse::<usize>() {
                        if count > highest_chapter_number {
                            println!("  Found Chapter Count metadata indicating {} chapters", count);
                            highest_chapter_number = count;
                        }
                    }
                }
            }
        }
        
        // Check for raw outline file
        let raw_outline_path = output_dir.join("raw_outline_output.txt");
        if raw_outline_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&raw_outline_path) {
                println!("  Debug: Found raw outline file with {} lines", content.lines().count());
                
                // Count chapters using a more flexible approach
                let chapter_count = content.lines()
                    .filter(|line| {
                        let trimmed = line.trim();
                        trimmed.starts_with("Chapter ") || 
                        trimmed.starts_with("CHAPTER ") || 
                        trimmed == "Introduction" || 
                        trimmed == "Conclusion" ||
                        (trimmed.contains("Chapter") && trimmed.contains(":"))
                    })
                    .count();
                
                println!("  Debug: First 100 chars of raw outline: {}", content.chars().take(100).collect::<String>());
                println!("  Debug: Found {} chapters in raw outline using flexible matching", chapter_count);
                
                // Print the first few chapter titles found
                let chapter_titles: Vec<_> = content.lines()
                    .filter(|line| {
                        let trimmed = line.trim();
                        trimmed.starts_with("Chapter ") || 
                        trimmed.starts_with("CHAPTER ") || 
                        trimmed == "Introduction" || 
                        trimmed == "Conclusion" ||
                        (trimmed.contains("Chapter") && trimmed.contains(":"))
                    })
                    .take(5)
                    .collect();
                
                println!("  Debug: First few chapter titles: {:?}", chapter_titles);
                
                if chapter_count > highest_chapter_number {
                    println!("  Found {} chapters in the raw outline file, but only {} loaded", 
                             chapter_count, highest_chapter_number);
                    highest_chapter_number = chapter_count;
                }
            }
        }
        
        if found_chapters > 0 || highest_chapter_number > book.chapters.len() {
            println!("Found evidence of {} chapters in total, but only {} were loaded.", 
                     highest_chapter_number, book.chapters.len());
            println!("Will resume from chapter {} to ensure no chapters are skipped.", 
                     book.chapters.len() + 1);
            
            // Update the starting chapter index based on what we've found
            start_chapter_index = book.chapters.len();
        }
    }
    
    // Process remaining chapters
    for (i, chapter_outline) in chapter_outlines.iter().enumerate().skip(start_chapter_index).take(max_chapters - start_chapter_index) {
        let chapter_number = start_chapter_index + i + 1;
        
        // Extract any chapter number from the title for clearer logging
        let title_display = chapter_outline.title.clone();
        
        println!("📄 Processing chapter {} (internal number): \"{}\" ({}/{} chapters)", 
                chapter_number, title_display, chapter_number, max_chapters);
        
        // Check for duplicate chapter titles
        if !seen_titles.insert(chapter_outline.title.clone()) {
            return Err(BookGeneratorError::DuplicateChapterTitle(chapter_outline.title.clone()));
        }
        
        // Check if this chapter already exists in the logs directory
        let chapter_file_pattern = format!("chapter_{}.json", chapter_number);
        let chapter_file_path = logs_dir.join(&chapter_file_pattern);
        
        // Also check for chapter generation files
        let chapter_gen_pattern = format!("chapter_generation_{}_", chapter_number);
        let mut chapter_exists = chapter_file_path.exists();
        
        // If the chapter file doesn't exist, check for chapter generation files
        if !chapter_exists && logs_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                for entry in entries.filter_map(|r| r.ok()) {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name() {
                        if let Some(file_name_str) = file_name.to_str() {
                            if file_name_str.contains(&chapter_gen_pattern) && 
                               !file_name_str.ends_with("_token_usage.txt") &&
                               !file_name_str.contains("token_usage") {
                                chapter_exists = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        if chapter_exists {
            println!("  📋 Found chapter file from current generation process - attempting to use it for chapter {}", chapter_number);
            
            // Try to load the chapter from the file
            if let Ok(content) = std::fs::read_to_string(&chapter_file_path) {
                // Try to parse the chapter from JSON
                if let Ok(mut existing_chapter) = serde_json::from_str::<Chapter>(&content) {
                    println!("  Successfully loaded chapter {} from file", chapter_number);
                    
                    // Note: We're no longer generating scenes here as part of chapter generation
                    // All scene generation is done in the dedicated scene generation phase
                    if existing_chapter.scenes.is_empty() {
                        println!("  Chapter {} has no scenes - scenes will be generated in Phase 4", chapter_number);
                    } else {
                        println!("  Chapter {} already has {} scenes", chapter_number, existing_chapter.scenes.len());
                        // Keep the existing scenes for now
                    }
                    
                    // Add the chapter to the book and continue the loop
                    book.chapters.push(existing_chapter);
                    continue;
                } else {
                    println!("  ⚠️ Could not parse chapter {} as JSON - will regenerate", chapter_number);
                }
            }
            
            // If we couldn't load the chapter, generate it
            println!("  ⚠️ Could not load existing chapter {} - regenerating", chapter_number);
        }
        
        // Generate the chapter
        let _previous_chapters = book.chapters.iter()
            .map(|ch| ch.outline.clone())
            .collect::<Vec<_>>();
        
        let chapter = generate_chapter(
            &mut book.context,
            config,
            chapter_number,
            chapter_outline.title.clone(),
            chapter_outline.clone(),
            output_dir,
            &token_tracker
        ).await?;
        
        // Save the chapter to a file
        let chapter_json = serde_json::to_string_pretty(&chapter)?;
        std::fs::write(&chapter_file_path, chapter_json)?;
        
        // Add the chapter to the book without generating scenes yet
        book.chapters.push(chapter);
    }
    
    let phase3_duration = phase3_start.elapsed();
    phase_timings.insert("Phase 3: Chapter Generation".to_string(), phase3_duration);
    println!("Phase 3 completed in {}", format_duration(phase3_duration));

    println!("\n🎬 Phase 4: Scene Generation");
    println!("-------------------------");
    let phase4_start = Instant::now();
    
    // Generate scenes for all chapters - only after all chapter outlines are complete
    for chapter in &mut book.chapters {
        println!("🎬 Generating scenes for chapter {}: \"{}\"", chapter.number, chapter.title);
        
        // Check if the chapter has scenes
        if chapter.scenes.is_empty() {
            println!("  Chapter {} has no scenes, generating them...", chapter.number);
            
            // First check if the chapter has scene outlines
            if chapter.outline.scenes.is_empty() {
                println!("  ⚠️ Chapter {} has no scene outlines in its chapter outline, checking book outline", chapter.number);
                
                // Get the chapter outline from the book outline
                let chapter_outline_from_book = book.context.outline.chapters.iter()
                    .find(|ch| ch.chapter_number == chapter.number)
                    .cloned();
                
                if let Some(ch_outline) = chapter_outline_from_book {
                    if !ch_outline.scenes.is_empty() {
                        println!("  📝 Using scene outlines from the book outline for chapter {} ({} scenes)", 
                                chapter.number, ch_outline.scenes.len());
                        chapter.outline.scenes = ch_outline.scenes.clone();
                    } else {
                        println!("  ⚠️ No scene outlines found in book outline for chapter {}, skipping scene generation", chapter.number);
                        continue;
                    }
                } else {
                    println!("  ⚠️ Could not find chapter {} in the book outline, skipping scene generation", chapter.number);
                    continue;
                }
            }
            
            // Generate scenes for this chapter
            println!("  🎬 Generating scenes for chapter {} based on {} scene outlines", 
                    chapter.number, chapter.outline.scenes.len());
            chapter.scenes = generate_scenes_for_chapter(
                &mut book.context,
                config,
                chapter,
                output_dir,
                &token_tracker
            ).await?;
            
            println!("  ✅ Generated {} scenes for chapter {}", chapter.scenes.len(), chapter.number);
        } else {
            println!("  Chapter {} already has {} scenes", chapter.number, chapter.scenes.len());
        }
    }
    
    let phase4_duration = phase4_start.elapsed();
    phase_timings.insert("Phase 4: Scene Generation".to_string(), phase4_duration);
    println!("Phase 4 completed in {}", format_duration(phase4_duration));

    println!("\n📝 Phase 5: Content Generation");
    println!("-------------------------");
    let phase5_start = Instant::now();
    
    // Generate content for all chapters and scenes
    for chapter in &mut book.chapters {
        println!("📄 Generating content for chapter {}: \"{}\"", chapter.number, chapter.title);
        
        // Initialize previous content vector for context
        let _previous_content: Vec<String> = Vec::new();
        
        // Generate content for all scenes in this chapter
        println!("  📝 Generating remaining content for chapter {}", chapter.number);
        generate_remaining_content(
            &mut book.context,
            config,
            chapter,
            output_dir,
            &token_tracker
        ).await?;
    }
    
    let phase5_duration = phase5_start.elapsed();
    phase_timings.insert("Phase 5: Content Generation".to_string(), phase5_duration);
    println!("Phase 5 completed in {}", format_duration(phase5_duration));
    
    // Calculate total generation time
    let total_duration = start_time.elapsed();
    println!("\n⏱️ Total book generation time: {}", format_duration(total_duration));
    
    // Store timing information in metadata
    let timing_info = format!(
        "Total: {}\nPhase 1: {}\nPhase 2: {}\nPhase 3: {}\nPhase 4: {}\nPhase 5: {}",
        format_duration(total_duration),
        format_duration(phase_timings["Phase 1: Initial Setup and Context"]),
        format_duration(phase_timings["Phase 2: Book Structure"]),
        format_duration(phase_timings["Phase 3: Chapter Generation"]),
        format_duration(phase_timings["Phase 4: Scene Generation"]),
        format_duration(phase_timings["Phase 5: Content Generation"])
    );
    update_metadata(output_dir, "Generation Time", &timing_info)?;
    
    // Attach timing information to the book for later use
    book.generation_time = Some(total_duration);
    book.phase_timings = Some(phase_timings.clone());
    
    if book.context.outline.chapters.is_empty() {
        return Err(BookGeneratorError::MissingContext("Book outline is missing".to_string()));
    }
    
    // Mark the book as completed based on content analysis
    book.is_completed = utils::file_utils::check_for_complete_book(output_dir);
    
    // Render the book to the output directory
    println!("\n📚 Phase 6: Book Rendering");
    println!("----------------------");
    println!("📚 Rendering book to output directory");
    crate::render::render_book(&book, output_dir, Some(&token_tracker)).await?;
    
    println!("✅ Book generation successful!\n");
    
    // Display token usage summary
    let prompt_tokens = token_tracker.get_prompt_tokens();
    let completion_tokens = token_tracker.get_completion_tokens();
    let total_tokens = prompt_tokens + completion_tokens;
    
    // Calculate cost using constants from logging.rs
    const CLAUDE_INPUT_PRICE: f64 = 3.0;  // $3 per 1M input tokens
    const CLAUDE_OUTPUT_PRICE: f64 = 15.0; // $15 per 1M output tokens
    let prompt_cost = (prompt_tokens as f64 / 1_000_000.0) * CLAUDE_INPUT_PRICE;
    let completion_cost = (completion_tokens as f64 / 1_000_000.0) * CLAUDE_OUTPUT_PRICE;
    let total_cost = prompt_cost + completion_cost;
    
    println!("\n🔤 Token Usage Summary:");
    println!("- Prompt Tokens: {}", prompt_tokens);
    println!("- Completion Tokens: {}", completion_tokens);
    println!("- Total Tokens: {}", total_tokens);
    println!("- Estimated Cost: ${:.2}", total_cost);
    
    Ok((book, token_tracker))
}

/// Generate scenes for a chapter
pub async fn generate_scenes_for_chapter(
    context: &mut Context,
    config: &Config,
    chapter: &mut Chapter,
    output_dir: &Path,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<Vec<Scene>> {
    let mut scenes = Vec::new();
    
    // First, check if we have existing scenes in the logs directory
    let logs_dir = output_dir.join("logs");
    if logs_dir.exists() {
        // Load existing scenes
        let existing_scenes = load_existing_scenes(context, config, chapter, output_dir, token_tracker).await?;
        
        if !existing_scenes.is_empty() {
            println!("Found {} existing scenes for chapter {}", existing_scenes.len(), chapter.number);
            
            // Create a set of scene numbers we already have
            let existing_scene_numbers: std::collections::HashSet<usize> = existing_scenes.iter()
                .map(|scene| scene.outline.number)
                .collect();
            
            // Add existing scenes to our collection
            scenes.extend(existing_scenes);
            
            // Only generate scenes that don't already exist
            for (i, scene_outline) in chapter.outline.scenes.iter().enumerate() {
                let scene_num = i + 1;
                
                if !existing_scene_numbers.contains(&scene_num) {
                    println!("Generating scene {} for chapter {}", scene_num, chapter.number);
                    
                    // Generate the scene
                    let scene = generate_new_scene(
                        context,
                        config,
                        chapter,
                        i, // Use zero-based index for scene generation
                        &scenes,
                        output_dir,
                        token_tracker,
                    ).await?;
                    
                    scenes.push(scene);
                } else {
                    println!("Skipping scene {} for chapter {} (already exists)", scene_num, chapter.number);
                }
            }
        } else {
            // No existing scenes, generate all of them
            println!("No existing scenes found for chapter {}. Generating all scenes...", chapter.number);
            
            for (i, _) in chapter.outline.scenes.iter().enumerate() {
                let scene_num = i + 1;
                println!("Generating scene {} for chapter {}", scene_num, chapter.number);
                
                let scene = generate_new_scene(
                    context,
                    config,
                    chapter,
                    i, // Use zero-based index for scene generation
                    &scenes,
                    output_dir,
                    token_tracker,
                ).await?;
                
                scenes.push(scene);
            }
        }
    } else {
        // No logs directory, generate all scenes
        println!("No logs directory found. Generating all scenes for chapter {}...", chapter.number);
        
        for (i, _) in chapter.outline.scenes.iter().enumerate() {
            let scene_num = i + 1;
            println!("Generating scene {} for chapter {}", scene_num, chapter.number);
            
            let scene = generate_new_scene(
                context,
                config,
                chapter,
                i, // Use zero-based index for scene generation
                &scenes,
                output_dir,
                token_tracker,
            ).await?;
            
            scenes.push(scene);
        }
    }
    
    // Sort scenes by scene number to ensure they're in the correct order
    scenes.sort_by_key(|scene| scene.outline.number);
    
    Ok(scenes)
}

async fn generate_new_scene(
    _context: &mut Context,
    _config: &Config,
    chapter: &Chapter,
    scene_index: usize,
    previous_scenes: &[Scene],
    output_dir: &Path,
    _token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<Scene> {
    // Check API availability before generating a new scene
    if !utils::api_verification::wait_for_api_availability(Some(Duration::from_secs(60))).await {
        println!("Warning: API still appears overloaded, but will attempt to continue with reduced request rate");
    }
    
    // Generate the scene
    Scene::generate(
        _context,
        _config,
        &chapter.outline,
        scene_index,
        previous_scenes,
        output_dir,
        _token_tracker,
    ).await
}

/// Generate content for scenes that don't have content yet
pub async fn generate_remaining_content(
    context: &mut Context,
    config: &Config,
    chapter: &mut Chapter,
    output_dir: &Path,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<()> {
    println!("🖋️ Generating content for chapter {}: \"{}\"", chapter.number, chapter.title);
    println!("📋 Found {} scenes in chapter {}", chapter.scenes.len(), chapter.number);
    
    // Check if we have any scenes to generate content for
    if chapter.scenes.is_empty() {
        println!("⚠️ No scenes found for chapter {}. Cannot generate content.", chapter.number);
        return Ok(());
    }
    
    // Changed from using default and field assignment to direct initialization
    let mut temp_summary = crate::book::tabula::TemporarySummary {
        content: format!(
            "Title: {}\nGenre: {}\nStyle: {}\nSynopsis: {}\n\n",
            context.title, context.genre, context.style, context.synopsis
        ),
        ..Default::default()
    };
    
    // Add chapter information
    temp_summary.content += &format!(
        "Chapter {}: {}\n{}\n\n",
        chapter.number, chapter.title, chapter.outline.description
    );
    
    // Add scene outlines
    temp_summary.content += "Scene Outlines:\n";
    for scene in &chapter.outline.scenes {
        temp_summary.content += &format!(
            "Scene {}: {}\n{}\n\n",
            scene.number, scene.title, scene.description
        );
    }
    
    // Add content from previous scenes
    let mut previous_content = String::new();
    for scene in &chapter.scenes {
        if !scene.content.text.is_empty() {
            previous_content += &format!(
                "Scene {}: {}\n{}\n\n",
                scene.outline.number, scene.title, scene.content.text
            );
        }
    }
    
    if !previous_content.is_empty() {
        temp_summary.content += "Previously Generated Content:\n";
        temp_summary.content += &previous_content;
        temp_summary.content += "\nHere is where we continue the story...\n";
    }
    
    // Save the temporary summary
    let temp_summary_file = output_dir.join("logs").join(
        format!("temporary_content_summary_ch{}.txt", chapter.number)
    );
    if let Some(parent) = temp_summary_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&temp_summary_file, &temp_summary.content)?;
    
    // Update the context with the temporary summary
    context.temporary_summary = temp_summary;
    
    // Generate content for each scene that doesn't have content yet
    let mut scene_indices_needing_content = Vec::new();
    
    // First, collect all scene indices that need content
    for (index, scene) in chapter.scenes.iter().enumerate() {
        if scene.content.text.is_empty() {
            scene_indices_needing_content.push(index);
        } else {
            println!("⏩ Skipping content generation for scene {} (already exists)", scene.outline.number);
        }
    }
    
    // Now process each scene that needs content
    for scene_index in scene_indices_needing_content {
        // Get scene information before we modify it
        let scene_number = chapter.scenes[scene_index].outline.number;
        let scene_title = chapter.scenes[scene_index].title.clone();
        let scene_description = chapter.scenes[scene_index].outline.description.clone();
        
        println!("🖋️ Generating content for scene {}: \"{}\"", scene_number, scene_title);
        
        // Create the prompt for content generation
        let mut prompt = String::new();
        
        // Add the temporary summary
        prompt += &context.temporary_summary.content;
        
        // Add the style guide
        prompt += &format!("\nWriting Style: {}\n\n", context.style.description);
        
        // Add the scene outline
        prompt += &format!(
            "Scene {} Outline: {}\n{}\n\n",
            scene_number, scene_title, scene_description
        );
        
        // Add instructions for content generation
        prompt += "Please write the content for this scene based on the outline and previous context. ";
        prompt += "The content should be engaging, follow the specified writing style, and maintain continuity with previous scenes. ";
        prompt += "Focus on showing rather than telling, include meaningful dialogue, and ensure character actions align with their established personalities.";
        
        // Generate the content using the correct function
        let content_text = crate::llm::generate(
            &config.model,
            &prompt,
            token_tracker
        ).await?;
        
        // Log the content generation with estimated token counts
        let prompt_tokens = prompt.len() / 4; // Rough estimate
        let completion_tokens = content_text.len() / 4; // Rough estimate
        
        let log_file = format!(
            "content_generation_ch{}_scene{}.txt",
            chapter.number, scene_number
        );
        
        crate::log_with_tracker(
            output_dir,
            &log_file,
            &content_text,
            prompt_tokens as u32,
            completion_tokens as u32,
            token_tracker,
        )?;
        
        // Update the scene with the generated content
        chapter.scenes[scene_index].content.text = content_text;
        chapter.scenes[scene_index].content.chapter_number = chapter.number;
        chapter.scenes[scene_index].content.scene_number = scene_number;
        
        println!("✅ Generated content for scene {}", scene_number);
        
        // Update the temporary summary with the newly generated content
        context.temporary_summary.content += &format!(
            "\nScene {}: {}\n{}\n\n",
            scene_number, scene_title, chapter.scenes[scene_index].content.text
        );
        context.temporary_summary.content += "Here is where we continue the story...\n";
        
        // Save the updated temporary summary
        std::fs::write(&temp_summary_file, &context.temporary_summary.content)?;
    }
    
    // Update the chapter content with all scene content
    let mut full_content = String::new();
    for scene in &chapter.scenes {
        full_content += &format!("## {}\n\n", scene.title);
        full_content += &scene.content.text;
        full_content += "\n\n";
    }
    chapter.content = full_content;
    
    println!("✅ Completed content generation for chapter {}: \"{}\"", chapter.number, chapter.title);
    
    Ok(())
}

pub fn log_with_tracker(
    output_dir: &Path,
    operation: &str,
    content: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    token_tracker: &utils::logging::TokenTracker,
) -> std::io::Result<()> {
    utils::logging::log_llm_output(
        output_dir,
        operation,
        content,
        prompt_tokens,
        completion_tokens,
        Some(token_tracker),
    )
}

/// Generate a single chapter, handling both new generation and loading from existing files
async fn generate_chapter(
    context: &mut Context,
    config: &Config,
    chapter_number: usize,
    chapter_title: String,
    chapter_outline: ChapterOutline,
    output_dir: &Path,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<Chapter> {
    // Check if this chapter already exists in the logs directory
    let logs_dir = output_dir.join("logs");
    let chapter_file = format!("chapter_{}.md", chapter_number);
    let chapter_file_path = logs_dir.join(&chapter_file);
    
    if chapter_file_path.exists() {
        println!("  📋 Found chapter file from current generation process - attempting to use it for chapter {}", chapter_number);
        
        // Try to load the chapter from the file
        if let Ok(content) = std::fs::read_to_string(&chapter_file_path) {
            // Try to parse the chapter from JSON
            if let Ok(mut existing_chapter) = serde_json::from_str::<Chapter>(&content) {
                println!("  Successfully loaded chapter {} from file", chapter_number);
                
                // Note: We're no longer generating scenes here as part of chapter generation
                // All scene generation is done in the dedicated scene generation phase
                if existing_chapter.scenes.is_empty() {
                    println!("  Chapter {} has no scenes - scenes will be generated in Phase 4", chapter_number);
                } else {
                    println!("  Chapter {} already has {} scenes", chapter_number, existing_chapter.scenes.len());
                    // Keep the existing scenes for now
                }
                
                // Return the existing chapter
                return Ok(existing_chapter);
            } else {
                println!("  ⚠️ Could not parse chapter {} as JSON - will regenerate", chapter_number);
            }
        } else {
            println!("  ⚠️ Could not read chapter file {} - will regenerate", chapter_number);
        }
    }
    
    // Get previous chapters for context
    let previous_chapters = context.outline.chapters.iter()
        .take(chapter_number - 1)
        .cloned()
        .collect::<Vec<_>>();
    
    // Generate the chapter using the Chapter's generate method
    // This properly creates a chapter outline with all the required context
    println!("  🔄 Generating full chapter {} with detailed outline", chapter_number);
    let chapter = Chapter::generate(
        chapter_number,
        &chapter_title,
        context,
        config,
        &previous_chapters,
        output_dir,
        token_tracker,
    ).await?;
    
    // Save the chapter to file
    let logs_dir = output_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;
    
    let chapter_file = logs_dir.join(format!("chapter_{}.json", chapter.number));
    let chapter_json = serde_json::to_string_pretty(&chapter)?;
    std::fs::write(&chapter_file, chapter_json)?;
    
    println!("  ✅ Generated new chapter {}: \"{}\"", chapter.number, chapter.title);
    Ok(chapter)
}

/// Process a chapter, generating it if it doesn't exist
async fn process_chapter(
    context: &mut Context,
    config: &Config,
    book: &mut Book,
    chapter_outline: &ChapterOutline,
    output_dir: &Path,
    token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<()> {
    println!("📖 Processing chapter {} (internal number {}): \"{}\"", chapter_outline.chapter_number, chapter_outline.chapter_number, chapter_outline.title);
    
    // Check for duplicate chapter titles
    if book.chapters.iter().any(|c| c.title == chapter_outline.title && c.number != chapter_outline.chapter_number) {
        println!("⚠️ Warning: Duplicate chapter title found: {}", chapter_outline.title);
        return Err(BookGeneratorError::DuplicateChapterTitle(chapter_outline.title.clone()));
    }
    
    // Check if chapter file exists in logs directory
    let logs_dir = output_dir.join("logs");
    let chapter_file = logs_dir.join(format!("chapter_{}.json", chapter_outline.chapter_number));
    
    // Also check for chapter generation files
    let chapter_gen_pattern = format!("chapter_generation_{}_", chapter_outline.chapter_number);
    let mut chapter_exists = false;
    
    if chapter_file.exists() {
        println!("  📋 Found existing chapter file: {}", chapter_file.display());
        chapter_exists = true;
    } else if let Ok(entries) = std::fs::read_dir(&logs_dir) {
        for entry in entries.filter_map(|r| r.ok()) {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    if file_name_str.contains(&chapter_gen_pattern) && !file_name_str.contains("token_usage") {
                        println!("  📋 Found existing chapter generation file: {}", path.display());
                        chapter_exists = true;
                        break;
                    }
                }
            }
        }
    }
    
    let mut chapter = if chapter_exists {
        // Try to load the chapter from file
        if chapter_file.exists() {
            match std::fs::read_to_string(&chapter_file) {
                Ok(content) => {
                    match serde_json::from_str::<Chapter>(&content) {
                        Ok(mut loaded_chapter) => {
                            println!("  ✅ Successfully loaded chapter {} from file", chapter_outline.chapter_number);
                            
                            // Check if the chapter has scenes
                            if loaded_chapter.scenes.is_empty() {
                                println!("  ⚠️ Loaded chapter has no scenes, generating scenes based on outline");
                                
                                // Generate scenes for this chapter
                                generate_scenes_for_chapter(
                                    context,
                                    config,
                                    &mut loaded_chapter,
                                    output_dir,
                                    token_tracker,
                                ).await?;
                                
                                // Save the updated chapter back to file
                                let chapter_json = serde_json::to_string_pretty(&loaded_chapter)?;
                                std::fs::write(&chapter_file, chapter_json)?;
                            }
                            
                            // Check if any scenes are missing content
                            let scenes_without_content = loaded_chapter.scenes.iter()
                                .filter(|s| s.content.text.is_empty())
                                .count();
                                
                            if scenes_without_content > 0 {
                                println!("  ⚠️ Found {} scenes without content, generating missing content", scenes_without_content);
                                
                                // Generate content for scenes that don't have it
                                generate_remaining_content(
                                    context,
                                    config,
                                    &mut loaded_chapter,
                                    output_dir,
                                    token_tracker,
                                ).await?;
                                
                                // Save the updated chapter back to file
                                let chapter_json = serde_json::to_string_pretty(&loaded_chapter)?;
                                std::fs::write(&chapter_file, chapter_json)?;
                            }
                            
                            loaded_chapter
                        },
                        Err(e) => {
                            println!("  ⚠️ Failed to parse chapter from file: {}", e);
                            
                            // Generate a new chapter
                            generate_chapter(
                                context,
                                config,
                                chapter_outline.chapter_number,
                                chapter_outline.title.clone(),
                                chapter_outline.clone(),
                                output_dir,
                                token_tracker,
                            ).await?
                        }
                    }
                },
                Err(e) => {
                    println!("  ⚠️ Failed to read chapter file: {}", e);
                    
                    // Generate a new chapter
                    generate_chapter(
                        context,
                        config,
                        chapter_outline.chapter_number,
                        chapter_outline.title.clone(),
                        chapter_outline.clone(),
                        output_dir,
                        token_tracker,
                    ).await?
                }
            }
        } else {
            // Generate a new chapter
            generate_chapter(
                context,
                config,
                chapter_outline.chapter_number,
                chapter_outline.title.clone(),
                chapter_outline.clone(),
                output_dir,
                token_tracker,
            ).await?
        }
    } else {
        // Generate a new chapter
        generate_chapter(
            context,
            config,
            chapter_outline.chapter_number,
            chapter_outline.title.clone(),
            chapter_outline.clone(),
            output_dir,
            token_tracker,
        ).await?
    };
    
    // Generate scenes for this chapter if needed
    if chapter.scenes.is_empty() {
        generate_scenes_for_chapter(
            context,
            config,
            &mut chapter,
            output_dir,
            token_tracker,
        ).await?;
    }
    
    // Generate content for scenes that don't have it
    generate_remaining_content(
        context,
        config,
        &mut chapter,
        output_dir,
        token_tracker,
    ).await?;
    
    // Save the chapter to file
    let chapter_json = serde_json::to_string_pretty(&chapter)?;
    std::fs::write(chapter_file, chapter_json)?;
    
    // Add the chapter to the book
    book.add_chapter(chapter);
    
    println!("✅ Completed processing chapter {}: \"{}\"", chapter_outline.chapter_number, chapter_outline.title);
    Ok(())
}

pub async fn load_existing_scenes(
    context: &mut Context,
    _config: &Config,
    chapter: &Chapter,
    output_dir: &Path,
    _token_tracker: &crate::utils::logging::TokenTracker,
) -> crate::error::Result<Vec<Scene>> {
    let logs_dir = output_dir.join("logs");
    if !logs_dir.exists() {
        println!("No logs directory found at {:?}", logs_dir);
        return Ok(Vec::new());
    }

    println!("Looking for scene files in {} for chapter {}", logs_dir.display(), chapter.number);
    
    // First pass: collect all scene outline and content files
    let mut scene_files = std::collections::HashMap::new();
    let mut content_files = std::collections::HashMap::new();
    
    if let Ok(entries) = std::fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { 
                continue;
            }
            
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            
            // Skip token usage and temporary files
            if filename.contains("token_usage") || filename.contains("temporary") {
                continue;
            }
            
            // Handle scene outline files
            if filename.contains(&format!("scene_generation_ch{}", chapter.number)) {
                if let Some(scene_num) = extract_scene_number(&filename) {
                    scene_files.entry(scene_num).or_insert(path.clone());
                }
            }
            
            // Handle content files - check for various patterns
            let content_patterns = [
                format!("content_generation_ch{}_scene", chapter.number),
                "scene_".to_string(), // Will be further filtered by scene number extraction
            ];
            
            for pattern in &content_patterns {
                if filename.contains(pattern) {
                    if let Some(scene_num) = extract_scene_number(&filename) {
                        content_files.entry(scene_num).or_insert(path.clone());
                    }
                }
            }
        }
    }
    
    println!("Found {} scene outline files and {} content files for chapter {}", 
             scene_files.len(), content_files.len(), chapter.number);
    
    if scene_files.is_empty() {
        println!("No scene files found for chapter {}", chapter.number);
        return Ok(Vec::new());
    }
    
    // Second pass: create scenes based on the files we found
    let mut scenes = Vec::new();
    
    // Check for content in src directory as well (final rendered content)
    let src_dir = output_dir.join("src");
    let chapter_file = src_dir.join(format!("chapter_{}.md", chapter.number));
    let mut chapter_content = String::new();
    if chapter_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&chapter_file) {
            chapter_content = content;
            println!("Found rendered chapter content for chapter {} ({} bytes)", 
                     chapter.number, chapter_content.len());
        }
    }
    
    for (scene_num, scene_path) in scene_files {
        // Read the scene outline file
        let scene_content = match std::fs::read_to_string(&scene_path) {
            Ok(content) => content,
            Err(e) => {
                println!("Error reading scene file {:?}: {}", scene_path, e);
                continue;
            }
        };
        
        // Find scene title and description
        let mut title = format!("Scene {}", scene_num);
        let mut description = String::new();
        
        // Extract title and description from the scene content
        for line in scene_content.lines() {
            if line.starts_with("Title:") || line.starts_with("# ") {
                title = line.trim_start_matches("Title:").trim_start_matches("# ").trim().to_string();
            } else if line.starts_with("Description:") || (description.is_empty() && !line.is_empty() && !line.starts_with('#')) {
                description = line.trim_start_matches("Description:").trim().to_string();
            }
        }
        
        // Create the scene outline
        let scene_outline = crate::book::outline::SceneOutline {
            number: scene_num,
            title: title.clone(),
            description: description.clone(),
        };
        
        // Enhanced content file finding, first check the standard locations
        let mut content_text = String::new();
        let mut found_content = false;
        
        // 1. First try direct match from content_files map
        if let Some(content_path) = content_files.get(&scene_num) {
            match std::fs::read_to_string(content_path) {
                Ok(text) => {
                    println!("Found content for chapter {} scene {} ({} bytes)", chapter.number, scene_num, text.len());
                    content_text = text;
                    found_content = true;
                },
                Err(e) => {
                    println!("Error reading content file {:?}: {}", content_path, e);
                }
            }
        }
        
        // 2. If not found, try the fallback finder
        if !found_content {
            if let Some(path) = find_existing_content_file(output_dir, chapter.number, scene_num) {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        println!("Found content for scene {} in non-standard file: {:?} ({} bytes)", 
                                 scene_num, path, text.len());
                        content_text = text;
                        found_content = true;
                    },
                    Err(e) => {
                        println!("Error reading content file {:?}: {}", path, e);
                    }
                }
            }
        }
        
        // 3. Check if we can extract content from the chapter file
        if !found_content && !chapter_content.is_empty() {
            // Try to extract this scene's content from the chapter file
            // This is a rough heuristic - extract content between scene markers or headers
            if let Some(scene_content) = extract_scene_from_chapter(&chapter_content, &title) {
                println!("Extracted content for scene {} from chapter file ({} bytes)", 
                         scene_num, scene_content.len());
                content_text = scene_content;
                found_content = true;
            }
        }
        
        // Create the content
        let content = crate::book::content::Content {
            text: content_text,
            chapter_number: chapter.number, 
            scene_number: scene_num,
        };
        
        // Create the scene
        let scene = crate::book::scene::Scene {
            outline: scene_outline,
            title,
            content,
        };
        
        // Add to our list of scenes
        scenes.push(scene);
    }
    
    // Sort scenes by scene number for proper ordering
    scenes.sort_by_key(|s| s.outline.number);
    
    // Report what we found
    let scenes_with_content = scenes.iter().filter(|s| !s.content.text.is_empty()).count();
    println!("Loaded {} scenes for chapter {}, {} with content", 
             scenes.len(), chapter.number, scenes_with_content);
    
    Ok(scenes)
}

/// Extract scene content from a chapter file
fn extract_scene_from_chapter(chapter_content: &str, scene_title: &str) -> Option<String> {
    // Look for scene title headers (## Scene title or # Scene title)
    let mut lines = chapter_content.lines().collect::<Vec<_>>();
    let mut scene_start = None;
    let mut scene_end = None;
    
    // Find scene boundaries
    for (i, line) in lines.iter().enumerate() {
        let clean_line = line.trim();
        if clean_line.contains(scene_title) && 
           (clean_line.starts_with("##") || clean_line.starts_with("# ")) {
            scene_start = Some(i);
            continue;
        }
        
        // If we found a scene start and now we found the next scene header, this is our end
        if scene_start.is_some() && scene_end.is_none() && 
           (clean_line.starts_with("##") || clean_line.starts_with("# ")) {
            scene_end = Some(i);
            break;
        }
    }
    
    // If we found a start but no end, the end is the end of the file
    if let (Some(start), Some(end)) = (scene_start, scene_end) {
        // Skip the header line
        let content = lines[start+1..end].join("\n");
        if !content.is_empty() {
            return Some(content);
        }
    }
    
    None
}

/// Load book from an existing directory
///
/// This function loads a book from an existing directory, scanning for
/// chapters and their scenes. It will set is_completed to true if the
/// book is detected as complete.
pub fn load_book_from_directory(output_dir: &Path) -> crate::error::Result<crate::book::Book> {
    let mut book = crate::book::Book::load_from_directory(output_dir)?;
    
    // Check if the book is complete by examining content files
    if utils::file_utils::check_for_complete_book(output_dir) {
        tracing::info!("Book detected as complete based on content analysis");
        book.is_completed = true;
    }
    
    Ok(book)
}


