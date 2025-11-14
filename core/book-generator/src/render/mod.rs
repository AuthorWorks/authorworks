#[allow(dead_code)]
use std::path::{Path, PathBuf};
use crate::book::Book;
use crate::book::chapter::Chapter;
use crate::book::scene::Scene;
use crate::error::{Result, BookGeneratorError};
use std::fs;
use mdbook::MDBook;
use mdbook::config::Config as MDBookConfig;
use crate::utils::statistics::BookStatistics;
use tracing::info;
use std::collections::HashSet;
use std::process::Command;
use crate::utils::file_utils::sanitize_filename;
use crate::utils::string_utils::{title_case, clean_chapter_title};
use crate::utils::logging::TokenTracker;

// Add the cover module
pub mod cover;

#[allow(dead_code)]
pub struct BookRenderer {
    output_dir: PathBuf,
}

#[allow(dead_code)]
impl BookRenderer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub async fn render(&self, book: &Book, token_tracker: Option<&TokenTracker>) -> Result<()> {
        // Calculate book statistics
        let statistics = match token_tracker {
            Some(tracker) => BookStatistics::with_tokens(book, tracker),
            None => BookStatistics::new(book),
        };
        
        // Save statistics to file and metadata without duplicating console output
        statistics.update_metadata(&self.output_dir)?;
        statistics.save_to_file(&self.output_dir)?;
        
        // Log statistics to tracing only
        info!("Book statistics: {} words, {} characters, {} pages, {} chapters, {} scenes",
            statistics.total_words,
            statistics.total_characters,
            statistics.estimated_pages,
            statistics.total_chapters,
            statistics.total_scenes
        );

        // Create the necessary directory structure
        let book_dir = self.output_dir.join("book");
        fs::create_dir_all(&book_dir)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to create book directory: {}", e)))?;

        // Create src directory as a backup for SUMMARY.md (for PDF/EPUB generation)
        let src_dir = self.output_dir.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to create src directory: {}", e)))?;

        // Create and write the SUMMARY.md file
        let summary_content = self.generate_summary(book);
        
        // Write SUMMARY.md to book directory
        fs::write(book_dir.join("SUMMARY.md"), &summary_content)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to write book/SUMMARY.md: {}", e)))?;
            
        // Also write SUMMARY.md to src directory (as a backup for PDF/EPUB generation)
        fs::write(src_dir.join("SUMMARY.md"), &summary_content)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to write src/SUMMARY.md: {}", e)))?;

        // Write chapter files (both to book and src directories)
        for (i, chapter) in book.chapters.iter().enumerate() {
            let chapter_content = self.format_chapter(chapter);
            let chapter_filename = format!("chapter_{}.md", i + 1);
            
            // Write to book directory
            fs::write(book_dir.join(&chapter_filename), &chapter_content)
                .map_err(|e| BookGeneratorError::Other(format!("Failed to write chapter file to book dir: {}", e)))?;
                
            // Also write to src directory (for PDF/EPUB generation)
            fs::write(src_dir.join(&chapter_filename), &chapter_content)
                .map_err(|e| BookGeneratorError::Other(format!("Failed to write chapter file to src dir: {}", e)))?;
        }

        // Add statistics to the book configuration
        let mut config = MDBookConfig::default();
        config.book.title = Some(format!("{} - A Novel", book.context.genre.name));
        config.book.authors = vec!["AI Book Generator".to_string()];
        config.book.description = Some(format!("{}\n\n{}", 
            book.context.synopsis.content.clone(),
            statistics
        ));
        
        // Configure the source directory to match our structure
        config.book.src = book_dir.strip_prefix(&self.output_dir)
            .unwrap_or_else(|_| Path::new("book"))
            .to_path_buf();
            
        // Add a post-rendering step to copy SUMMARY.md back if it gets deleted
        println!("📚 MDBook configuration set up, rendering book...");

        // Create a new MDBook instance with our configuration
        let md_book = MDBook::load_with_config(&self.output_dir, config)
            .map_err(|e| BookGeneratorError::Other(format!("Failed to load MDBook: {}", e)))?;
        md_book.build().map_err(|e| BookGeneratorError::Other(format!("Failed to build MDBook: {}", e)))?;
        
        // After MDBook rendering is complete, ensure SUMMARY.md still exists in required locations
        // This helps if MDBook moved or removed the file during its build process
        let summary_exists_in_book = book_dir.join("SUMMARY.md").exists();
        let summary_exists_in_src = src_dir.join("SUMMARY.md").exists();
        
        if !summary_exists_in_book {
            println!("Restoring SUMMARY.md in book directory (for PDF/EPUB generation)");
            if summary_exists_in_src {
                // Copy from src to book
                fs::copy(src_dir.join("SUMMARY.md"), book_dir.join("SUMMARY.md"))
                    .map_err(|e| BookGeneratorError::Other(format!("Failed to restore SUMMARY.md: {}", e)))?;
            } else {
                // Regenerate and write
                fs::write(book_dir.join("SUMMARY.md"), &summary_content)
                    .map_err(|e| BookGeneratorError::Other(format!("Failed to regenerate SUMMARY.md: {}", e)))?;
            }
        }
        
        if !summary_exists_in_src {
            println!("Restoring SUMMARY.md in src directory (for PDF/EPUB generation)");
            if summary_exists_in_book {
                // Copy from book to src
                fs::copy(book_dir.join("SUMMARY.md"), src_dir.join("SUMMARY.md"))
                    .map_err(|e| BookGeneratorError::Other(format!("Failed to restore SUMMARY.md: {}", e)))?;
            } else {
                // Regenerate and write
                fs::write(src_dir.join("SUMMARY.md"), &summary_content)
                    .map_err(|e| BookGeneratorError::Other(format!("Failed to regenerate SUMMARY.md: {}", e)))?;
            }
        }
        
        // Create a statistics file in the rendered book directory
        let html_stats_path = self.output_dir.join("book").join("html").join("statistics.html");
        if let Some(parent) = html_stats_path.parent() {
            if parent.exists() {
                let html_content = format!(
                    "<!DOCTYPE html>\n<html>\n<head>\n<title>Book Statistics</title>\n</head>\n<body>\n<h1>Book Statistics</h1>\n<pre>{}</pre>\n</body>\n</html>",
                    statistics
                );
                fs::write(html_stats_path, html_content)
                    .map_err(|e| BookGeneratorError::Other(format!("Failed to write statistics HTML: {}", e)))?;
            }
        }

        Ok(())
    }

    fn generate_summary(&self, book: &Book) -> String {
        let mut summary = String::from("# Summary\n\n");
        
        // Add statistics to the summary
        summary.push_str("- [Statistics](statistics.html)\n");
        
        // Add chapters to the summary
        for (i, chapter) in book.chapters.iter().enumerate() {
            summary.push_str(&format!("- [{}](chapter_{}.md)\n", chapter.title, i + 1));
        }
        summary
    }

    fn format_chapter(&self, chapter: &Chapter) -> String {
        let mut content = format!("# {}\n\n", chapter.title);
        content.push_str(&format!("*{}*\n\n", chapter.outline.description));
        for scene in chapter.scenes.iter() {
            content.push_str(&self.format_scene(scene));
        }
        content
    }

    fn format_scene(&self, scene: &Scene) -> String {
        let mut content = String::new();
        content.push_str(&scene.content.to_string());
        content.push_str("\n\n");
        content
    }
}

/// Generates PDF and EPUB versions of the book
///
/// This function creates a single markdown file with all chapters,
/// properly formatted for conversion to PDF and EPUB using pandoc.
pub fn generate_pdf_and_epub(output_dir: &Path, book_title: &str, author: &str) -> Result<()> {
    // Look for SUMMARY.md in the book directory or src directory
    let book_dir = output_dir.join("book");
    let src_dir = output_dir.join("src");
    let summary_path = book_dir.join("SUMMARY.md");
    let src_summary_path = src_dir.join("SUMMARY.md");
    
    let (summary_path_to_use, content_dir) = if summary_path.exists() {
        (summary_path, book_dir.clone())
    } else if src_summary_path.exists() {
        (src_summary_path, src_dir.clone())
    } else {
        println!("Warning: SUMMARY.md not found in book or src directory. PDF and EPUB generation skipped.");
        println!("Expected paths: {:?} or {:?}", summary_path, src_summary_path);
        return Ok(());
    };

    println!("🔍 Phase 6: Locating book content and preparing for export");
    println!("Using SUMMARY.md from: {:?}", summary_path_to_use);

    // Check if we have a cover image
    let cover_image_path = output_dir.join("images").join("cover.png");
    let cover_svg_path = output_dir.join("images").join("cover.svg");
    
    let (cover_path, cover_path_str) = if cover_image_path.exists() {
        println!("📚 Using existing cover image: {:?}", cover_image_path);
        (cover_image_path.clone(), cover_image_path.display().to_string())
    } else if cover_svg_path.exists() {
        println!("📚 Using existing SVG cover image: {:?}", cover_svg_path);
        (cover_svg_path.clone(), cover_svg_path.display().to_string())
    } else {
        println!("📚 No cover image found, proceeding without one");
        (PathBuf::new(), String::new())
    };

    // Create a temporary file that includes all chapter content
    let temp_file_path = output_dir.join("full_book_content.md");
    println!("📝 Phase 6: Concatenating all chapters into a single document");
    
    // Create a YAML metadata block for pandoc
    let mut full_content = String::new();
    
    // Enhanced YAML metadata block for better pandoc processing
    full_content.push_str("---\n");
    full_content.push_str(&format!("title: {}\n", book_title));
    full_content.push_str(&format!("author: {}\n", author));
    full_content.push_str("rights: Copyright © 2025\n");
    full_content.push_str("language: en-US\n");
    full_content.push_str("documentclass: book\n");
    full_content.push_str("classoption: oneside\n");
    full_content.push_str("papersize: a5\n");
    full_content.push_str("geometry: margin=1in\n");
    full_content.push_str("fontsize: 11pt\n");
    full_content.push_str("mainfont: Palatino\n");
    full_content.push_str("sansfont: Helvetica\n");
    full_content.push_str("monofont: Courier\n");
    full_content.push_str("linkcolor: black\n");
    full_content.push_str("urlcolor: black\n");
    full_content.push_str("toc-title: Contents\n");
    full_content.push_str("---\n\n");
    
    // Create a professional title page with proper formatting
    full_content.push_str(&format!("<h1 class=\"title\">{}</h1>\n\n", book_title));
    full_content.push_str(&format!("<p class=\"author\">{}</p>\n\n", author));
    full_content.push_str("<p class=\"copyright\">Copyright © 2025</p>\n\n");
    full_content.push_str("\\newpage\n\n");
    
    // Add a proper copyright page that will appear after the TOC
    full_content.push_str("\\pagenumbering{gobble}\n\n"); // Hide page number for copyright page
    full_content.push_str("<div class=\"copyright-page\">\n");
    full_content.push_str(&format!("<p>{}</p>\n\n", book_title));
    full_content.push_str("<p>Published by P.I.E. LLC</p>\n\n");
    full_content.push_str("<p>Copyright © 2025 by ");
    full_content.push_str(author);
    full_content.push_str("</p>\n\n");
    full_content.push_str("<p>All rights reserved. No part of this publication may be reproduced, distributed, or transmitted in any form or by any means, including photocopying, recording, or other electronic or mechanical methods, without the prior written permission of the publisher, except in the case of brief quotations embodied in critical reviews and certain other noncommercial uses permitted by copyright law.</p>\n\n");
    full_content.push_str("<p>First Edition: ");
    // Add current date in Month Year format
    full_content.push_str(&chrono::Local::now().format("%B %Y").to_string());
    full_content.push_str("</p>\n\n");
    full_content.push_str("<p>ISBN: </p>\n\n");
    full_content.push_str("<p>Printed in the United States of America</p>\n");
    full_content.push_str("</div>\n\n");
    full_content.push_str("\\newpage\n\n");
    full_content.push_str("\\pagenumbering{arabic}\n\n"); // Restore page numbering
    
    // Read SUMMARY.md to get chapter order
    let summary_content = fs::read_to_string(&summary_path_to_use)?;
    
    // Track chapters to avoid duplicates in TOC
    let mut processed_chapters = HashSet::new();
    
    // First pass: Extract chapter information - filter out anything that's not a chapter
    let mut chapters = Vec::new();
    for line in summary_content.lines() {
        if let Some(chapter_info) = line.trim().strip_prefix("- [").and_then(|s| s.split_once("](")) {
            let chapter_title = chapter_info.0;
            let chapter_file_name = chapter_info.1.trim_end_matches(')');
            
            // Only process top-level entries (chapters, not scenes)
            // Simplify the boolean expression
            if !(line.starts_with("  ") || 
                 chapter_title.contains("Statistics") ||
                 chapter_title.contains("Style") ||
                 chapter_title.contains("Writing") ||
                 chapter_title == "Outline" || 
                 chapter_title == "Book Outline" ||
                 chapter_file_name.contains("statistics.html")) {
                chapters.push((chapter_title, chapter_file_name));
            }
        }
    }
    
    // Second pass: Process each chapter
    for (chapter_idx, (chapter_title, chapter_file_name)) in chapters.iter().enumerate() {
        // Try multiple locations for the chapter file
        let chapter_path = content_dir.join(chapter_file_name);
        let alt_chapter_path = if content_dir == book_dir {
            src_dir.join(chapter_file_name)
        } else {
            book_dir.join(chapter_file_name)
        };
        
        let chapter_file_to_use = if chapter_path.exists() {
            println!("Adding chapter from primary location: {:?}", chapter_path);
            chapter_path
        } else if alt_chapter_path.exists() {
            println!("Adding chapter from alternative location: {:?}", alt_chapter_path);
            alt_chapter_path
        } else {
            println!("Warning: Chapter file not found in any location: {:?} or {:?}", 
                     chapter_path, alt_chapter_path);
            continue;
        };
            
        // Only add if we haven't processed this chapter yet
        if processed_chapters.insert(chapter_title) {
            // Add chapter heading (level 1) with chapter number
            let chapter_num = chapter_idx + 1; // Start from chapter 1 (not 0)
            
            // Clean the chapter title to prevent duplicates and inconsistent naming
            let clean_chapter_title = clean_chapter_title(chapter_title);
            
            // Format chapter heading properly with consistent numbering
            // Check if the clean title already starts with "Chapter X" to avoid duplication
            let chapter_heading = if clean_chapter_title.starts_with(&format!("Chapter {}", chapter_num)) {
                format!("# {}\n\n", clean_chapter_title)
            } else {
                format!("# Chapter {}: {}\n\n", chapter_num, clean_chapter_title)
            };
            full_content.push_str(&chapter_heading);
            
            // Read and add chapter content
            match fs::read_to_string(&chapter_file_to_use) {
                Ok(chapter_content) => {
                    // Process chapter content to remove duplicate headings and adjust heading levels
                    let processed_content = process_chapter_content(&chapter_content);
                    
                    // Check if we got any content after processing
                    if processed_content.trim().is_empty() {
                        println!("Warning: No content extracted from chapter file: {:?}", chapter_file_to_use);
                        // If no content was extracted, try adding the raw content with minimal processing
                        let lines: Vec<&str> = chapter_content.lines().collect();
                        let mut simple_content = String::new();
                        
                        // Skip the first heading if it exists (we already added our own)
                        let mut skip_first_heading = true;
                        for line in lines {
                            if line.trim().starts_with("# ") && skip_first_heading {
                                skip_first_heading = false;
                                continue;
                            }
                            simple_content.push_str(line);
                            simple_content.push('\n');
                        }
                        
                        full_content.push_str(&simple_content);
                    } else {
                        full_content.push_str(&processed_content);
                    }
                    
                    full_content.push_str("\n\n\\newpage\n\n"); // Add page break between chapters
                },
                Err(e) => {
                    println!("Error reading chapter file: {:?} - {}", chapter_file_to_use, e);
                    continue;
                }
            }
        }
    }
    
    // Write the full content to a temporary file
    println!("Writing content to temporary file: {:?}", temp_file_path);
    fs::write(&temp_file_path, full_content)?;
    
    // Verify the file was created
    if !temp_file_path.exists() {
        println!("Error: Failed to create temporary file: {:?}", temp_file_path);
        return Err(BookGeneratorError::Other(format!("Failed to create temporary file: {:?}", temp_file_path)));
    }
    println!("Created temporary file: {:?}", temp_file_path);

    // Create a CSS file for EPUB formatting with improved styling
    let css_content = r#"
/* Standard e-book formatting based on industry norms */
body {
    margin: 5%;
    text-align: justify;
    font-size: 1em;
    line-height: 1.5;
    font-family: 'Crimson Pro', serif;
    color: #333;
}

h1 {
    font-size: 1.5em;
    line-height: 1.2;
    text-align: center;
    margin: 2em 0 1em 0;
    font-weight: normal;
    page-break-before: always;
    page-break-after: avoid;
}

h2 {
    font-size: 1.3em;
    text-align: left;
    margin: 1.5em 0 0.5em 0;
    font-weight: normal;
    page-break-after: avoid;
}

h3 {
    font-size: 1.1em;
    margin: 1em 0 0.5em 0;
    font-weight: normal;
    font-style: italic;
    page-break-after: avoid;
}

p {
    margin: 0;
    text-indent: 1.5em;
    widows: 2;
    orphans: 2;
}

/* First paragraph after heading should not be indented */
h1 + p, h2 + p, h3 + p {
    text-indent: 0;
}

/* Title page formatting */
section.titlepage {
    text-align: center;
    page-break-after: always;
}

.title {
    font-size: 2em;
    line-height: 1.2;
    margin: 0 0 1.5em 0;
}

.author {
    font-size: 1.3em;
    margin: 0 0 3em 0;
}

.copyright {
    font-size: 0.9em;
    margin: 3em 0 0 0;
}

/* Table of Contents formatting */
nav#toc {
    page-break-before: always;
    page-break-after: always;
}

nav#toc h1 {
    font-size: 1.8em;
    text-align: center;
    margin: 3em 0 2em 0;
    font-family: 'Palatino', serif;
    font-weight: normal;
}

nav#toc ol {
    list-style-type: none;
    margin: 0 0 2em 0;
    padding: 0;
}

nav#toc ol li {
    margin: 0.7em 0;
    text-indent: 0;
    font-size: 1em;
    line-height: 1.4;
}

nav#toc ol li a {
    text-decoration: none;
    color: #333;
    display: block;
    padding: 0.2em 0;
}

/* Style for blockquotes and emphasis */
blockquote {
    margin: 1em 1.5em;
    font-style: italic;
    text-indent: 0;
}

em, i {
    font-style: italic;
}

strong, b {
    font-weight: bold;
}

/* Fix for scene titles in TOC - hide scene numbers */
.toc-section-number {
    display: none;
}

/* Ensure chapter titles are properly formatted in TOC */
nav#toc ol li a.chapter {
    font-weight: bold;
    font-size: 1.1em;
    margin-top: 0.8em;
    font-family: 'Palatino', serif;
}

/* Hide scene prefixes in TOC entries */
nav#toc ol li a[href^='#scene'] {
    font-weight: normal;
    padding-left: 1.5em;
    font-size: 0.95em;
    color: #555;
}

/* Remove text before colon in TOC entries */
nav#toc ol li a span.toc-text {
    position: relative;
}

/* Copyright page styling */
div.copyright-page {
    margin: 3em 0;
    text-align: left;
    page-break-before: always;
    page-break-after: always;
}

div.copyright-page p {
    margin: 1em 0;
    text-indent: 0;
    font-size: 0.9em;
    line-height: 1.4;
}
"#;
    let css_path = output_dir.join("epub.css");
    fs::write(&css_path, css_content)?;
    println!("Created CSS file for EPUB formatting: {:?}", css_path);

    // Get absolute paths
    let absolute_temp_path = fs::canonicalize(&temp_file_path)?;
    println!("Absolute path of temporary file: {:?}", absolute_temp_path);

    // Sanitize book title for filenames
    let sanitized_title = sanitize_filename(book_title);
    
    // Format title properly - capitalize and remove hyphens for display
    let display_title = book_title.split('-')
        .map(|word| word.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");
    
    // Generate PDF with improved formatting
    let pdf_output = format!("{}.pdf", sanitized_title);
    println!("📄 Phase 6: Generating PDF output file '{}'", pdf_output);
    
    // Create longer-lived formatted strings for PDF generation
    let title_metadata = format!("--metadata=title:{}", display_title);
    let author_metadata = format!("--metadata=author:{}", author);
    
    // Prepare cover metadata and paths if needed
    let cover_metadata = if !cover_path_str.is_empty() && cover_path.exists() {
        format!("cover-image={}", cover_path_str)
    } else {
        String::new()
    };
    
    // Prepare absolute cover path for EPUB if it exists
    let absolute_cover_path_str = if !cover_path_str.is_empty() && cover_path.exists() {
        match fs::canonicalize(&cover_path) {
            Ok(abs_path) => abs_path.to_string_lossy().to_string(),
            Err(_) => cover_path_str.clone()
        }
    } else {
        String::new()
    };
    
    // Use more appropriate PDF formatting settings for books 
    let mut pdf_args = vec![
        "-o", &pdf_output,
        absolute_temp_path.to_str().unwrap(),
        "--pdf-engine=xelatex",
        "--toc",
        "--toc-depth=2",               // Include chapters and scenes in TOC
        "--standalone",
        &title_metadata,
        &author_metadata,
        "--metadata=lang:en-US",
        "--variable=documentclass:book",
        "--variable=classoption:oneside",
        "--variable=papersize:a5",
        "--variable=fontsize:11pt",
        "--variable=geometry:margin=1in",
        "--variable=mainfont:Palatino",
        "--variable=sansfont:Helvetica",
        "--variable=monofont:Courier",
        "--variable=linkcolor:black",
        "--variable=urlcolor:black",
        "--variable=toc-title:Contents",
        "--no-highlight",              // Disable code highlighting for a book
        "--top-level-division=chapter", // Treat top-level headers as chapters
        "--wrap=none",                 // Don't wrap text
        "--section-divs",              // Wrap sections in div tags for better styling
    ];
    
    // Add cover image to PDF if it exists
    if !cover_metadata.is_empty() {
        pdf_args.push("--metadata");
        pdf_args.push(&cover_metadata);
    }
    
    let pdf_result = Command::new("pandoc")
        .args(&pdf_args)
        .current_dir(output_dir)
        .output();
    
    match pdf_result {
        Ok(output) => {
            if output.status.success() {
                println!("✅ PDF generation completed successfully: {}", pdf_output);
            } else {
                println!("PDF generation failed with status: {}", output.status);
                println!("Error output: {}", String::from_utf8_lossy(&output.stderr));
                
                // Print the command that failed for debugging
                println!("Failed command: pandoc {}", pdf_args.join(" "));
            }
        },
        Err(e) => println!("PDF generation failed: {}", e),
    }

    // Generate EPUB with improved formatting
    let epub_output = format!("{}.epub", sanitized_title);
    println!("📱 Phase 6: Generating EPUB output file '{}'", epub_output);
    
    // Create longer-lived formatted strings for EPUB generation
    let epub_title_metadata = format!("--metadata=title:{}", display_title);
    let epub_author_metadata = format!("--metadata=creator:{}", author);
    
    // Build the EPUB command arguments - updated for current pandoc version
    let mut epub_args = vec![
        "-o".to_string(), epub_output.clone(),
        absolute_temp_path.to_str().unwrap().to_string(),
        "--from=markdown".to_string(),
        "--to=epub3".to_string(),                  // Use EPUB3 format for better compatibility
        "--toc".to_string(),                       // Include table of contents
        "--toc-depth=2".to_string(),              // Include chapters and scenes in TOC
        "--standalone".to_string(),
        epub_title_metadata,
        epub_author_metadata,
        "--metadata=publisher:P.I.E. LLC".to_string(),       // Set publisher to P.I.E. LLC
        "--metadata=lang:en-US".to_string(),
        "--metadata=rights:Copyright © 2025".to_string(),
        "--css=epub.css".to_string(),              // Specify the CSS file for styling
        "--split-level=1".to_string(),             // Replace deprecated --epub-chapter-level with --split-level
        "--no-highlight".to_string(),              // Disable code highlighting for fiction
    ];
    
    // Create a basic metadata.xml file for EPUB accessibility
    let metadata_xml = format!(r#"<dc:language>en-US</dc:language>
<dc:title>{}</dc:title>
<dc:creator>{}</dc:creator>
<dc:publisher>P.I.E. LLC</dc:publisher>
<dc:rights>Copyright © 2025</dc:rights>
<dc:identifier id="pub-id">urn:uuid:{}</dc:identifier>
"#, display_title, author, uuid::Uuid::new_v4());
    
    fs::write(output_dir.join("metadata.xml"), metadata_xml)?;
    
    // Add cover image if it exists - use absolute path to ensure it's found
    if !cover_path_str.is_empty() && cover_path.exists() {
        // Get absolute path to cover image to avoid path resolution issues
        match fs::canonicalize(&cover_path) {
            Ok(absolute_cover_path) => {
                // Convert to string to avoid lifetime issues
                let absolute_path_str = absolute_cover_path.to_string_lossy().to_string();
                
                // Use the correct format for cover image in EPUB with absolute path
                epub_args.push("--epub-cover-image".to_string());
                epub_args.push(absolute_path_str.clone());
                println!("📚 Adding cover image to EPUB: {}", absolute_path_str);
            },
            Err(e) => {
                println!("⚠️ Warning: Failed to get absolute path for cover image: {}. Using relative path.", e);
                epub_args.push("--epub-cover-image".to_string());
                epub_args.push(cover_path_str.clone());
            }
        }
    } else {
        println!("📚 No cover image found for EPUB, proceeding without one");
    }
    
    // Run pandoc to generate EPUB
    println!("📚 Generating EPUB with pandoc...");
    let epub_result = Command::new("pandoc")
        .args(&epub_args)
        .current_dir(output_dir)
        .status();
    
    match epub_result {
        Ok(status) => {
            if status.success() {
                println!("✅ EPUB generation completed successfully: {}", epub_output);
            } else {
                println!("EPUB generation failed with status: {}", status);
                println!("Failed command: pandoc {}", epub_args.join(" "));
            }
        },
        Err(e) => println!("EPUB generation failed: {}", e),
    }
    
    // Clean up temporary files
    if temp_file_path.exists() {
        match fs::remove_file(&temp_file_path) {
            Ok(_) => println!("Temporary file removed: {:?}", temp_file_path),
            Err(e) => println!("Warning: Could not remove temporary file: {:?} - {}", temp_file_path, e),
        }
    }
    
    Ok(())
}

/// Process chapter content to remove duplicate headings and format appropriately
fn process_chapter_content(content: &str) -> String {
    // Helper function to extract clean scene title
    fn extract_scene_title(heading_text: &str) -> Option<String> {
        // Check if it starts with "Scene" followed by a number
        if heading_text.starts_with("Scene") {
            // If there's a colon, take everything after it and trim
            if let Some(colon_pos) = heading_text.find(':') {
                let title = heading_text.split_at(colon_pos + 1).1.trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
            
            // If there's a space after "SceneX", take everything after it
            // First find where the number ends
            let mut number_end_pos = 0;
            for (i, c) in heading_text.chars().enumerate().skip(5) { // Skip "Scene"
                if !c.is_ascii_digit() {
                    number_end_pos = i;
                    break;
                }
            }
            
            if number_end_pos > 0 && number_end_pos < heading_text.len() {
                let title = heading_text.split_at(number_end_pos).1.trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
        
        // If it doesn't start with "Scene" or we couldn't extract a title, return None
        None
    }
    
    // Helper function to check if a line is metadata that should be skipped
    fn is_metadata_line(line: &str) -> bool {
        const METADATA_PATTERNS: [&str; 30] = [
            "*Prompt:", "Braindump:", "## Braindump", "## Genre", 
            "## Style", "Style:", "Genre:", "Recommended Writing Style",
            "Writing Style", "Detailed Outline", "*Note:", "Character List",
            "Theme:", "Setting:", "Tone:", "POV:", "Point of View:",
            "## Synopsis", "## Characters", "## Plot", "## Setting",
            "Chapter Statistics", "Word Count:", "## Word Count", 
            "## Feedback", "Feedback:", "Notes:", "## Notes", "*Topics:", 
            "# EMPRESS CYPRESS", // Filter out book title in all caps
        ];
        
        METADATA_PATTERNS.iter().any(|pattern| 
            line.contains(pattern) || 
            line.to_lowercase().contains(&pattern.to_lowercase())
        ) || line == "=====" || line.trim() == line.trim().to_uppercase() && line.trim().len() > 10
    }
    
    // Helper function to process a line and add it to the result
    fn process_line(line: &str, result: &mut String) {
        let trimmed = line.trim();
        
        // Process headings to maintain proper hierarchy and clean up scene titles
        if trimmed.starts_with("## ") {
            let heading_text = trimmed.trim_start_matches("## ");
            
            // Process scene headings
            if let Some(scene_title) = extract_scene_title(heading_text) {
                // Add the cleaned scene title as an h2 heading
                result.push_str(&format!("## {}\n", scene_title));
            } else {
                // For other h2 headings, keep them as is
                result.push_str(line);
                result.push('\n');
            }
        } else if trimmed.starts_with("# ") && (trimmed.contains("Chapter") || trimmed.contains("CHAPTER")) {
            // Skip duplicate chapter headings - don't add to result
        } else {
            // Regular content, add it as is
            result.push_str(line);
            result.push('\n');
        }
    }
    
    // First, find the chapter description line (usually starts with *)
    let mut chapter_description = String::new();
    for line in content.lines() {
        if line.trim().starts_with('*') && !line.contains("Prompt:") && !line.contains("Note:") {
            chapter_description = line.trim().to_string();
            break;
        }
    }
    
    // Try standard processing approach first
    let mut result = String::new();
    let mut first_heading_skipped = false;
    let mut in_code_block = false;
    let mut content_started = false;
    
    // Add the chapter description if found
    if !chapter_description.is_empty() {
        result.push_str(&chapter_description);
        result.push_str("\n\n");
    }
    
    // Process content line by line
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Handle code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }
        
        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }
        
        // Skip the first main heading (chapter title) as we've already added it
        if !first_heading_skipped && trimmed.starts_with("# ") {
            first_heading_skipped = true;
            continue;
        }
        
        // Check if this is a metadata line to skip
        if is_metadata_line(trimmed) {
            continue;
        }
        
        // Skip empty lines until we find content
        if !content_started && trimmed.is_empty() {
            continue;
        }
        
        // We've found content
        content_started = true;
        
        // Process the line
        process_line(line, &mut result);
    }
    
    // If we didn't get any content, try a more lenient approach
    if result.trim().is_empty() || result.trim() == chapter_description.trim() {
        result.clear();
        
        // Add the chapter description if found
        if !chapter_description.is_empty() {
            result.push_str(&chapter_description);
            result.push_str("\n\n");
        }
        
        // Just include everything except the first heading and obvious metadata
        let mut first_heading_skipped = false;
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Skip the first heading
            if !first_heading_skipped && trimmed.starts_with("# ") {
                first_heading_skipped = true;
                continue;
            }
            
            // Skip obvious metadata (exact matches only in lenient mode)
            if is_metadata_line(trimmed) && trimmed == trimmed.trim() {
                continue;
            }
            
            // Process the line
            process_line(line, &mut result);
        }
    }
    
    // Trim any excessive blank lines and return
    result.trim().to_string()
}

#[allow(dead_code)]
pub async fn render_book(book: &Book, output_dir: &Path, token_tracker: Option<&TokenTracker>) -> Result<()> {
    // Generate cover image first
    println!("🎨 Generating cover image for the book...");
    let cover_result = cover::generate_cover_image(book, output_dir).await;
    
    match cover_result {
        Ok(cover_path) => {
            println!("✅ Cover image generated successfully: {:?}", cover_path);
        },
        Err(e) => {
            println!("⚠️ Failed to generate AI cover image: {}. Falling back to SVG generation.", e);
            match cover::generate_fallback_cover(book, output_dir) {
                Ok(fallback_path) => println!("✅ Fallback cover generated: {:?}", fallback_path),
                Err(e) => println!("⚠️ Failed to generate fallback cover: {}. Proceeding without cover.", e)
            }
        }
    }
    
    // Proceed with normal rendering
    let renderer = BookRenderer::new(output_dir.to_path_buf());
    renderer.render(book, token_tracker).await
}