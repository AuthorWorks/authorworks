use std::fs;
use std::env;
use std::path::{Path, PathBuf};

use book_generator::{
    Config,
    generate_book, generate_book_with_dir, render_book, generate_remaining_content,
    utils::{
        input::get_user_input,
        api_verification::verify_anthropic_api_key,
        logging::TokenTracker,
        statistics::BookStatistics,
        file_utils::check_for_complete_book,
    },
    load_existing_scenes,
    generate_pdf_and_epub,
    load_book_from_directory,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    
    // Check if we're just generating PDF/EPUB for an existing book
    if args.len() > 1 && args[1] == "pdf" {
        return handle_pdf_command(&args[1..]).await;
    }
    
    // Check if we're resuming a specific book
    if args.len() > 2 && args[1] == "resume" {
        let book_name = &args[2];
        return handle_resume_command(book_name).await;
    }

    let config = Config::default();

    // Verify the API key before proceeding
    verify_anthropic_api_key().await.map_err(|e| Box::<dyn std::error::Error>::from(format!("{}", e)))?;

    println!("Welcome to the AI Book Generator!");

    // Ask if user wants to resume from an existing directory
    println!("Do you want to resume from an existing output directory? (y/N)");
    let resume_input = get_user_input("")?.to_lowercase();
    
    let output_dir: PathBuf;
    let book_title: String;
    
    if resume_input == "y" {
        // List available output directories
        let output_base = Path::new("output");
        if !output_base.exists() {
            println!("No output directory exists yet. Starting a new book generation.");
            fs::create_dir_all(output_base)?;
        } else {
            let entries = fs::read_dir(output_base)?;
            let mut dirs = Vec::new();
            
            // Collect all directory entries
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    dirs.push(entry.path());
                }
            }
            
            // Sort directories alphabetically by name
            dirs.sort_by(|a, b| {
                let a_name = a.file_name().unwrap_or_default().to_string_lossy();
                let b_name = b.file_name().unwrap_or_default().to_string_lossy();
                a_name.cmp(&b_name)
            });
            
            println!("Available output directories:");
            for (i, dir_path) in dirs.iter().enumerate() {
                println!("{}. {}", i + 1, dir_path.file_name().unwrap().to_string_lossy());
            }
            
            if dirs.is_empty() {
                println!("No existing output directories found. Starting a new book generation.");
            } else {
                println!("Enter the number of the directory to resume from (or 0 for a new book):");
                let dir_num = get_user_input("")?.parse::<usize>().unwrap_or(0);
                
                if dir_num > 0 && dir_num <= dirs.len() {
                    output_dir = dirs[dir_num - 1].clone();
                    book_title = output_dir.file_name().unwrap().to_string_lossy().to_string();
                    
                    println!("Resuming book generation for: {}", book_title);
                    
                    // Check if the book is already complete using our improved detection
                    if check_for_complete_book(&output_dir) {
                        println!("📚 Book '{}' is already fully generated", book_title);
                        println!("Proceeding with Phase 6: Rendering and file generation");
                        
                        // Load the existing book using the improved loader that checks for completeness
                        let mut book = match load_book_from_directory(&output_dir) {
                            Ok(loaded_book) => {
                                println!("Successfully loaded existing book with {} chapters", loaded_book.chapters.len());
                                loaded_book
                            },
                            Err(e) => {
                                println!("Error loading book from directory: {}", e);
                                println!("Attempting to regenerate the book");
                                let (regenerated_book, _) = generate_book_with_dir(book_title.clone(), &config, &output_dir, config.auto_generate).await?;
                                regenerated_book
                            }
                        };
                        
                        // Create a token tracker for loaded books
                        let token_tracker = TokenTracker::new();
                        
                        // Check if the book was loaded from a completed state
                        if book.is_completed {
                            println!("\n📚 Book '{}' is already fully generated, proceeding directly to rendering", book_title);
                            
                            // Display book statistics with token information
                            let statistics = BookStatistics::with_tokens(&book, &token_tracker);
                            
                            // Save statistics to file and update metadata
                            statistics.save_to_file(&output_dir)?;
                            statistics.update_metadata(&output_dir)?;
                            
                            println!("\n{}", statistics);
                            
                            // Render the book using MDBook
                            render_book(&book, &output_dir, Some(&token_tracker)).await?;
                            
                            // Generate PDF and EPUB files
                            println!("📚 Phase 6: Generating PDF and EPUB files");
                            let author_name = "l3o".to_string(); // Default for resume command
                            generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;
                            
                            println!("✅ SUCCESS: Book rendering and export completed!");
                            println!("   Phase 6: Book Rendering & Export ✓");
                            println!("\nFinal output directory: {:?}", output_dir);
                            
                            return Ok(());
                        } else {
                            // Generate the book if not already completed
                            let (generated_book, token_tracker) = generate_book_with_dir(book_title.clone(), &config, &output_dir, config.auto_generate).await?;
                            book = generated_book;
                            
                            // Display book statistics with token information
                            let statistics = if let (Some(total_time), Some(phase_timings)) = (book.generation_time, book.phase_timings.clone()) {
                                BookStatistics::with_timings_and_tokens(&book, total_time, phase_timings, &token_tracker)
                            } else {
                                BookStatistics::with_tokens(&book, &token_tracker)
                            };
                            
                            // Save statistics to file and update metadata
                            statistics.save_to_file(&output_dir)?;
                            statistics.update_metadata(&output_dir)?;
                            
                            println!("\n{}", statistics);
                            
                            // Render the book using MDBook
                            render_book(&book, &output_dir, Some(&token_tracker)).await?;
                            
                            // Generate PDF and EPUB files
                            println!("📚 Phase 6: Generating PDF and EPUB files");
                            let author_name = "l3o".to_string(); // Default for resume command
                            generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;
                            
                            println!("✅ SUCCESS: All 6 phases of book generation completed!");
                            println!("   Phase 1: Initial Setup and Context ✓");
                            println!("   Phase 2: Book Structure ✓");
                            println!("   Phase 3: Chapter Generation ✓");
                            println!("   Phase 4: Scene Generation ✓");
                            println!("   Phase 5: Content Generation ✓");
                            println!("   Phase 6: Book Rendering & Export ✓");
                            println!("\nFinal output directory: {:?}", output_dir);
                            
                            return Ok(());
                        }
                    }
                    
                    // If the book is not complete, use the new function that accepts an output directory
                    let (book, token_tracker) = generate_book_with_dir(book_title.clone(), &config, &output_dir, config.auto_generate).await?;
                    
                    // Check if book is marked as completed (loaded from directory with completion flag)
                    if book.is_completed {
                        println!("\n📚 Book '{}' is already fully generated, proceeding directly to rendering", book_title);
                        
                        // Display book statistics with token information
                        let statistics = BookStatistics::with_tokens(&book, &token_tracker);
                        
                        // Save statistics to file and update metadata
                        statistics.save_to_file(&output_dir)?;
                        statistics.update_metadata(&output_dir)?;
                        
                        println!("\n{}", statistics);
                        
                        // Render the book using MDBook
                        render_book(&book, &output_dir, Some(&token_tracker)).await?;
                        
                        // Generate PDF and EPUB files
                        println!("📚 Phase 6: Generating PDF and EPUB files");
                        let author_name = "l3o".to_string(); // Default for resume command
                        generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;
                        
                        println!("✅ SUCCESS: Book rendering and export completed!");
                        println!("   Phase 6: Book Rendering & Export ✓");
                        println!("\nFinal output directory: {:?}", output_dir);
                        
                        return Ok(());
                    }
                    
                    // Display book statistics with token information
                    let statistics = if let (Some(total_time), Some(phase_timings)) = (book.generation_time, book.phase_timings.clone()) {
                        BookStatistics::with_timings_and_tokens(&book, total_time, phase_timings, &token_tracker)
                    } else {
                        BookStatistics::with_tokens(&book, &token_tracker)
                    };
                    
                    // Save statistics to file and update metadata
                    statistics.save_to_file(&output_dir)?;
                    statistics.update_metadata(&output_dir)?;
                    
                    println!("\n{}", statistics);
                    
                    // Render the book using MDBook
                    render_book(&book, &output_dir, Some(&token_tracker)).await?;
                    
                    // Generate PDF and EPUB files
                    println!("📚 Phase 6: Generating PDF and EPUB files");
                    let author_name = "l3o".to_string(); // Default for resume command
                    generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;
                    
                    println!("✅ SUCCESS: All 6 phases of book generation completed!");
                    println!("   Phase 1: Initial Setup and Context ✓");
                    println!("   Phase 2: Book Structure ✓");
                    println!("   Phase 3: Chapter Generation ✓");
                    println!("   Phase 4: Scene Generation ✓");
                    println!("   Phase 5: Content Generation ✓");
                    println!("   Phase 6: Book Rendering & Export ✓");
                    println!("\nFinal output directory: {:?}", output_dir);
                    
                    return Ok(());
                }
            }
        }
    }
    
    // If not resuming, proceed with new book generation
    println!("Do you want to input a book title? (Y/n)");
    let input_title = get_user_input("")?.to_lowercase();
    book_title = if input_title != "n" {
        get_user_input("Enter the book title: ")?
    } else {
        "Untitled Book".to_string()
    };

    // Add author prompt
    println!("Do you want to input an author name? (Y/n)");
    let input_author = get_user_input("")?.to_lowercase();
    let author_name = if input_author != "n" {
        let author_input = get_user_input("Enter the author name (default: l3o): ")?;
        if author_input.trim().is_empty() {
            "l3o".to_string()
        } else {
            author_input
        }
    } else {
        "l3o".to_string()
    };

    // Ask if user wants to generate all components automatically
    println!("Do you want to generate all book components automatically? (y/N)");
    println!("(This will generate title, braindump, genre, style, characters, synopsis, and book outline automatically)");
    let auto_generate = get_user_input("")?.to_lowercase() == "y";

    // Create a custom config with the auto_generate flag
    let mut custom_config = config.clone();
    custom_config.auto_generate = auto_generate;

    output_dir = Path::new("output").join(book_generator::sanitize_directory_name(&book_title));
    
    let (book, token_tracker) = generate_book(book_title.clone(), &custom_config).await?;

    // Display book statistics with timing and token information
    let statistics = if let (Some(total_time), Some(phase_timings)) = (book.generation_time, book.phase_timings.clone()) {
        BookStatistics::with_timings_and_tokens(&book, total_time, phase_timings, &token_tracker)
    } else {
        BookStatistics::with_tokens(&book, &token_tracker)
    };
    
    // Save statistics to file and update metadata
    statistics.save_to_file(&output_dir)?;
    statistics.update_metadata(&output_dir)?;
    
    println!("\n{}", statistics);

    // Render the book using MDBook
    render_book(&book, &output_dir, Some(&token_tracker)).await?;

    // Generate PDF and EPUB files
    println!("📚 Phase 6: Generating PDF and EPUB files");
    generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;

    println!("✅ SUCCESS: All 6 phases of book generation completed!");
    println!("   Phase 1: Initial Setup and Context ✓");
    println!("   Phase 2: Book Structure ✓");
    println!("   Phase 3: Chapter Generation ✓");
    println!("   Phase 4: Scene Generation ✓");
    println!("   Phase 5: Content Generation ✓");
    println!("   Phase 6: Book Rendering & Export ✓");
    println!("\nFinal output directory: {:?}", output_dir);

    Ok(())
}

async fn handle_pdf_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut book_title = String::new();
    let mut output_dir = PathBuf::new();
    let mut author_name = "l3o".to_string();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-t" | "--title" => {
                if i + 1 < args.len() {
                    book_title = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-a" | "--author" => {
                if i + 1 < args.len() {
                    author_name = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    
    if output_dir.as_os_str().is_empty() {
        println!("Error: Output directory must be specified with -o or --output");
        return Ok(());
    }
    
    if book_title.is_empty() {
        // Try to infer book title from directory name
        if let Some(dir_name) = output_dir.file_name() {
            book_title = dir_name.to_string_lossy().to_string();
        } else {
            println!("Error: Book title must be specified with -t or --title");
            return Ok(());
        }
    }
    
    // Check if we need to create a book_complete.flag file
    // Enhanced logic to better handle interrupted generations
    let src_dir = output_dir.join("src");
    let logs_dir = output_dir.join("logs");
    
    // Check for required directories
    if !src_dir.exists() && !logs_dir.exists() {
        println!("Error: Neither src nor logs directory found in {}", output_dir.display());
        println!("This doesn't appear to be a valid book directory.");
        return Ok(());
    }
    
    // Verify there's actual content to work with
    let mut has_content = false;
    
    // Check for content in src directory (rendered markdown)
    if src_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("chapter_") && filename.ends_with(".md") {
                        has_content = true;
                        break;
                    }
                }
            }
        }
    }
    
    // If no rendered content, check logs for raw content files
    if !has_content && logs_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.contains("content_generation") && 
                       !filename.contains("token_usage") {
                        has_content = true;
                        break;
                    }
                }
            }
        }
    }
    
    if !has_content {
        println!("Warning: No chapter content found in directory {}", output_dir.display());
        println!("You may need to complete book generation before generating PDF/EPUB files.");
        
        // Ask if user wants to continue anyway
        println!("Continue anyway? (y/N)");
        let input = book_generator::utils::input::get_user_input("")?;
        if input.trim().to_lowercase() != "y" {
            return Ok(());
        }
    }
    
    // Create a flag file if book appears complete but no flag exists
    if check_for_complete_book(&output_dir) {
        // Content exists but no flag file - create it
        let completion_flag = output_dir.join("book_complete.flag");
        if !completion_flag.exists() {
            println!("📌 Book content exists but flag file is missing. Creating flag file...");
            fs::write(&completion_flag, format!("Book generation completed at {}", chrono::Local::now()))
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create completion flag: {}", e)))?;
            println!("✅ Created missing book_complete.flag file");
        }
    }
    
    println!("Generating PDF and EPUB for '{}' by '{}' in directory: {:?}", book_title, author_name, output_dir);
    generate_pdf_and_epub(&output_dir, &book_title, &author_name)?;
    
    Ok(())
}

async fn handle_resume_command(book_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the config
    let config = Config::default();

    // Verify the API key before proceeding
    verify_anthropic_api_key().await.map_err(|e| Box::<dyn std::error::Error>::from(format!("{}", e)))?;

    println!("Welcome to the AI Book Generator!");
    
    // Construct the output directory path
    let output_base = Path::new("output");
    let output_dir = output_base.join(book_name);
    
    // Check if the directory exists
    if !output_dir.exists() {
        println!("Error: Book '{}' not found in output directory.", book_name);
        println!("Available books:");
        
        // List available books
        if let Ok(entries) = fs::read_dir(output_base) {
            let mut dirs = Vec::new();
            
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    dirs.push(entry.path());
                }
            }
            
            // Sort directories alphabetically
            dirs.sort_by(|a, b| {
                let a_name = a.file_name().unwrap_or_default().to_string_lossy();
                let b_name = b.file_name().unwrap_or_default().to_string_lossy();
                a_name.cmp(&b_name)
            });
            
            for dir_path in dirs {
                println!("- {}", dir_path.file_name().unwrap().to_string_lossy());
            }
        }
        
        return Err(Box::<dyn std::error::Error>::from(format!("Book '{}' not found", book_name)));
    }
    
    println!("📚 Resuming book generation for '{}'", book_name);
    
    // Check if the book is already complete using our improved detection
    if check_for_complete_book(&output_dir) {
        println!("📚 Book '{}' is already fully generated", book_name);
        println!("Proceeding with Phase 6: Rendering and file generation");
        
        // Load the existing book using the improved loader that checks for completeness
        let mut book = match load_book_from_directory(&output_dir) {
            Ok(loaded_book) => {
                println!("Successfully loaded existing book with {} chapters", loaded_book.chapters.len());
                loaded_book
            },
            Err(e) => {
                println!("Error loading book from directory: {}", e);
                println!("Falling back to regular book generation process");
                let (book, token_tracker) = generate_book_with_dir(book_name.to_string(), &config, &output_dir, config.auto_generate).await?;
                
                // Display book statistics with timing information
                let statistics = if let (Some(total_time), Some(phase_timings)) = (book.generation_time, book.phase_timings.clone()) {
                    BookStatistics::with_timings_and_tokens(&book, total_time, phase_timings, &token_tracker)
                } else {
                    BookStatistics::with_tokens(&book, &token_tracker)
                };
                
                // Save statistics to file and update metadata
                statistics.save_to_file(&output_dir)?;
                statistics.update_metadata(&output_dir)?;
                
                println!("\n{}", statistics);
                
                // Render the book using MDBook
                render_book(&book, &output_dir, Some(&token_tracker)).await?;
                
                // Generate PDF and EPUB files
                println!("📚 Phase 6: Generating PDF and EPUB files");
                let author_name = "l3o".to_string(); // Default for resume command
                generate_pdf_and_epub(&output_dir, book_name, &author_name)?;
                
                println!("✅ SUCCESS: All 6 phases of book generation completed!");
                println!("   Phase 1: Initial Setup and Context ✓");
                println!("   Phase 2: Book Structure ✓");
                println!("   Phase 3: Chapter Generation ✓");
                println!("   Phase 4: Scene Generation ✓");
                println!("   Phase 5: Content Generation ✓");
                println!("   Phase 6: Book Rendering & Export ✓");
                println!("\nFinal output directory: {:?}", output_dir);
                
                return Ok(());
            }
        };
        
        // Check if the book has any chapters
        if book.chapters.is_empty() {
            println!("No chapters found in the book. Attempting to load outline and create chapters...");
            
            // Try to load the outline from outline.json
            let outline_file = output_dir.join("outline.json");
            if outline_file.exists() {
                if let Ok(outline_content) = std::fs::read_to_string(&outline_file) {
                    if let Ok(outline) = serde_json::from_str::<book_generator::Outline>(&outline_content) {
                        println!("Found outline with {} chapters", outline.chapters.len());
                        
                        // Create chapters from the outline
                        for (i, chapter_outline) in outline.chapters.iter().enumerate() {
                            let chapter_num = i + 1;
                            let chapter = book_generator::Chapter {
                                number: chapter_num,
                                title: chapter_outline.title.clone(),
                                outline: chapter_outline.clone(),
                                scenes: Vec::new(),
                                content: String::new(),
                            };
                            book.add_chapter(chapter);
                        }
                        
                        println!("Created {} chapters from outline", book.chapters.len());
                        
                        // Update the book's outline
                        book.context.outline = outline;
                    }
                }
            }
        }
        
        // Initialize token tracker
        let token_tracker = TokenTracker::new();
        
        // Load scenes for each chapter
        println!("Loading scenes for {} chapters...", book.chapters.len());
        let mut needs_content_generation = false;
        let mut total_scenes_found = 0;
        
        for chapter in &mut book.chapters {
            println!("Looking for scene files for chapter {}", chapter.number);
            
            // Load existing scenes for this chapter
            let scenes = load_existing_scenes(&mut book.context, &config, chapter, &output_dir, &token_tracker).await?;
            
            if !scenes.is_empty() {
                println!("Found {} scenes for chapter {}", scenes.len(), chapter.number);
                total_scenes_found += scenes.len();
                
                // Add scenes to the chapter
                for scene in scenes {
                    // Check if this scene needs content generation
                    if scene.content.text.is_empty() {
                        needs_content_generation = true;
                    }
                    chapter.scenes.push(scene);
                }
            } else {
                println!("Found 0 scenes for chapter {}", chapter.number);
            }
        }
        
        // Check if we found any scenes
        if total_scenes_found > 0 {
            println!("Found a total of {} scenes across all chapters", total_scenes_found);
            
            // Check if any chapters have scenes that need content generation
            if needs_content_generation {
                println!("Some scenes need content generation. Proceeding to content generation phase...");
                
                // Generate content for each chapter
                for chapter in &mut book.chapters {
                    if !chapter.scenes.is_empty() {
                        // Check if any scenes in this chapter need content
                        let needs_content = chapter.scenes.iter().any(|scene| scene.content.text.is_empty());
                        
                        if needs_content {
                            println!("Generating content for chapter {}: {}", chapter.number, chapter.title);
                            generate_remaining_content(&mut book.context, &config, chapter, &output_dir, &token_tracker).await?;
                        }
                    }
                }
            } else {
                println!("All scenes already have content. Proceeding to rendering...");
            }
        } else {
            println!("No scenes found for any chapter. Generating scene outlines and content...");
            
            // Generate scenes and content for each chapter
            for chapter in &mut book.chapters {
                if !chapter.outline.scenes.is_empty() {
                    println!("Generating scenes for chapter {}: {}", chapter.number, chapter.title);
                    
                    // Generate scenes for this chapter
                    let scenes = book_generator::generate_scenes_for_chapter(
                        &mut book.context,
                        &config,
                        chapter,
                        &output_dir,
                        &token_tracker
                    ).await?;
                    
                    // Add scenes to the chapter
                    chapter.scenes = scenes;
                    
                    // Generate content for this chapter
                    println!("Generating content for chapter {}: {}", chapter.number, chapter.title);
                    generate_remaining_content(&mut book.context, &config, chapter, &output_dir, &token_tracker).await?;
                }
            }
        }
        
        // Display book statistics with token usage information
        let statistics = BookStatistics::with_tokens(&book, &token_tracker);
        
        // Save statistics to file and update metadata
        statistics.save_to_file(&output_dir)?;
        statistics.update_metadata(&output_dir)?;
        
        println!("\n{}", statistics);
        
        // Render the book using MDBook
        render_book(&book, &output_dir, Some(&token_tracker)).await?;
        
        // Generate PDF and EPUB files
        println!("📚 Phase 6: Generating PDF and EPUB files");
        let author_name = "l3o".to_string(); // Default for resume command
        generate_pdf_and_epub(&output_dir, book_name, &author_name)?;
        
        println!("✅ SUCCESS: All 6 phases of book generation completed!");
        println!("   Phase 1: Initial Setup and Context ✓");
        println!("   Phase 2: Book Structure ✓");
        println!("   Phase 3: Chapter Generation ✓");
        println!("   Phase 4: Scene Generation ✓");
        println!("   Phase 5: Content Generation ✓");
        println!("   Phase 6: Book Rendering & Export ✓");
        println!("\nFinal output directory: {:?}", output_dir);
        
        return Ok(());
    }
    
    // If we didn't skip to content generation, continue with the normal flow
    // Use the function that accepts an output directory
    let (book, token_tracker) = generate_book_with_dir(book_name.to_string(), &config, &output_dir, config.auto_generate).await?;
    
    // Display book statistics with timing and token information
    let statistics = if let (Some(total_time), Some(phase_timings)) = (book.generation_time, book.phase_timings.clone()) {
        BookStatistics::with_timings_and_tokens(&book, total_time, phase_timings, &token_tracker)
    } else {
        BookStatistics::with_tokens(&book, &token_tracker)
    };
    
    // Save statistics to file and update metadata
    statistics.save_to_file(&output_dir)?;
    statistics.update_metadata(&output_dir)?;
    
    println!("\n{}", statistics);
    
    // Render the book using MDBook
    render_book(&book, &output_dir, Some(&token_tracker)).await?;
    
    // Generate PDF and EPUB files
    println!("📚 Phase 6: Generating PDF and EPUB files");
    let author_name = "l3o".to_string(); // Default for resume command
    generate_pdf_and_epub(&output_dir, book_name, &author_name)?;
    
    println!("✅ SUCCESS: All 6 phases of book generation completed!");
    println!("   Phase 1: Initial Setup and Context ✓");
    println!("   Phase 2: Book Structure ✓");
    println!("   Phase 3: Chapter Generation ✓");
    println!("   Phase 4: Scene Generation ✓");
    println!("   Phase 5: Content Generation ✓");
    println!("   Phase 6: Book Rendering & Export ✓");
    println!("\nFinal output directory: {:?}", output_dir);
    
    Ok(())
}