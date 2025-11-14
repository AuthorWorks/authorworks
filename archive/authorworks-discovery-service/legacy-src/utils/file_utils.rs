use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use chrono::Local;
use serde_json;

/// Extracts scene number from a filename that follows the pattern "scene_X_*"
/// where X is the scene number.
pub fn extract_scene_number(filename: &str) -> Option<usize> {
    // Try exact pattern matching first for common format
    if let Some(scene_part) = filename.split('_')
        .find(|part| part.starts_with("scene")) {
        if let Ok(scene_num) = scene_part.trim_start_matches("scene").parse::<usize>() {
            return Some(scene_num);
        }
    }
    
    // Fallback to regex-like pattern for files that don't follow the exact pattern
    // Look for "scene" followed by digits at a word boundary
    let mut current_pos = 0;
    while let Some(pos) = filename[current_pos..].find("scene") {
        let start_pos = current_pos + pos + "scene".len();
        
        // If we've reached the end of the string, break
        if start_pos >= filename.len() {
            break;
        }
        
        // Extract digits following "scene"
        let mut end_pos = start_pos;
        while end_pos < filename.len() && filename[end_pos..end_pos+1].chars().next().is_some_and(|c| c.is_ascii_digit()) {
            end_pos += 1;
        }
        
        // If we found digits, parse them
        if end_pos > start_pos {
            if let Ok(scene_num) = filename[start_pos..end_pos].parse::<usize>() {
                return Some(scene_num);
            }
        }
        
        // Move past this occurrence for the next iteration
        current_pos = start_pos;
    }
    
    None
}

/// Finds an existing content file for a given chapter and scene number in the provided directory.
pub fn find_existing_content_file(output_dir: &Path, chapter_number: usize, scene_number: usize) -> Option<PathBuf> {
    let logs_dir = output_dir.join("logs");
    if !logs_dir.exists() {
        return None;
    }
    
    if let Ok(entries) = fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            
            let filename = path.file_name()?.to_str()?;
            
            // Skip temporary files and token usage files
            if filename.contains("temporary") || filename.contains("token_usage") {
                continue;
            }
            
            // Match specific content file patterns, including those with timestamps
            if (filename.contains(&format!("content_generation_ch{}_scene{}", chapter_number, scene_number)) ||
                filename.contains(&format!("content_generation_ch{}_scene{}.", chapter_number, scene_number)) || // Match files with extensions
                filename.contains(&format!("scene_{}_content_ch{}", scene_number, chapter_number)) ||
                filename.contains(&format!("scene_{}_content", scene_number))) &&
               (path.extension().and_then(|s| s.to_str()) == Some("md") || 
                path.extension().and_then(|s| s.to_str()) == Some("txt")) {
                
                // If we found a content file, log it for debugging
                tracing::debug!(
                    "Found content file for chapter {} scene {}: {:?}", 
                    chapter_number, scene_number, path
                );
                
                return Some(path);
            }
        }
    }
    
    // Try the new pattern that uses timestamps (e.g., content_generation_ch1_scene1.txt_20250226_211404.txt)
    if let Ok(entries) = fs::read_dir(&logs_dir) {
        let pattern1 = format!("content_generation_ch{}_scene{}.txt_", chapter_number, scene_number);
        
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            
            let filename = path.file_name()?.to_str()?;
            
            if filename.starts_with(&pattern1) {
                tracing::debug!(
                    "Found timestamped content file for chapter {} scene {}: {:?}", 
                    chapter_number, scene_number, path
                );
                return Some(path);
            }
        }
    }
    
    None
}

/// Checks if all necessary files exist for a complete book in the output directory.
pub fn check_for_complete_book(output_dir: &Path) -> bool {
    // Step 1: Check if rendered book files exist in the book directory
    let book_dir = output_dir.join("book");
    if book_dir.exists() {
        if let Ok(book_entries) = fs::read_dir(&book_dir) {
            let mut html_files = Vec::new();
            
            for entry in book_entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "html" && path.file_name().and_then(|n| n.to_str()).is_some_and(|name| name.starts_with("chapter_")) {
                        html_files.push(path);
                    }
                }
            }
            
            if !html_files.is_empty() {
                tracing::info!("Found {} rendered HTML chapter files in book directory", html_files.len());
                // If we have HTML chapter files, consider the book complete
                return true;
            }
        }
    }

    // Step 2: Look for the metadata file to extract book outline
    let metadata_file = output_dir.join("metadata.md");
    if !metadata_file.exists() {
        tracing::info!("No metadata file found at {:?}", metadata_file);
        return false;
    }

    // Check if outline exists in metadata to determine expected chapter count
    let outline_section = crate::read_metadata_section_safe(output_dir, "Book Outline");
    let mut expected_chapter_count = 0;
    
    if let Some(outline) = outline_section {
        // Count chapters in the outline (simple heuristic)
        expected_chapter_count = outline.lines()
            .filter(|l| l.starts_with("Chapter ") || l.starts_with("CHAPTER ") || 
                        (l.contains("Chapter") && l.contains(":")))
            .count();
        
        tracing::info!("Found {} chapters in book outline", expected_chapter_count);
        
        // If no chapters found in outline, use a fallback count
        if expected_chapter_count == 0 {
            expected_chapter_count = 10; // Default fallback
        }
    } else {
        tracing::info!("No book outline found in metadata");
        return false;
    }

    // Step 3: Count actual chapter, scene, and content files
    let mut chapter_files = Vec::new();
    let mut scene_files = std::collections::HashMap::new();
    let mut content_files = std::collections::HashMap::new();
    
    // Check src directory for chapter files
    let src_dir = output_dir.join("src");
    if src_dir.exists() {
        if let Ok(src_entries) = fs::read_dir(&src_dir) {
            for entry in src_entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("chapter_") && filename.ends_with(".md") {
                        // Extract chapter number
                        if let Some(num_str) = filename.strip_prefix("chapter_").and_then(|s| s.strip_suffix(".md")) {
                            if let Ok(num) = num_str.parse::<usize>() {
                                chapter_files.push((num, path.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check logs directory for scene and content files
    let logs_dir = output_dir.join("logs");
    if logs_dir.exists() {
        if let Ok(log_entries) = fs::read_dir(&logs_dir) {
            for entry in log_entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip token usage and temporary files
                    if filename.contains("token_usage") || filename.contains("temporary") {
                        continue;
                    }
                    
                    // Extract chapter and scene numbers for matching
                    let chapter_num = extract_chapter_number(filename);
                    let scene_num = extract_scene_number(filename);
                    
                    if chapter_num.is_some() && scene_num.is_some() {
                        let ch_num = chapter_num.unwrap();
                        let sc_num = scene_num.unwrap();
                        
                        // Track scene generation files
                        if filename.contains("scene_generation") {
                            scene_files.entry(ch_num)
                                .or_insert_with(Vec::new)
                                .push((sc_num, path.clone()));
                        }
                        
                        // Track content generation files
                        if filename.contains("content_generation") || 
                           (filename.contains("scene_") && filename.contains("content")) {
                            content_files.entry(ch_num)
                                .or_insert_with(Vec::new)
                                .push((sc_num, path.clone()));
                        }
                    }
                }
            }
        }
    }
    
    // Sort chapter files
    chapter_files.sort_by_key(|(num, _)| *num);
    
    // If no chapters found, book is not complete
    if chapter_files.is_empty() {
        tracing::info!("No chapter files found");
        return false;
    }
    
    // Count how many chapters have scenes and content
    let mut chapters_with_scenes = 0;
    let mut chapters_with_content = 0;
    
    for (ch_num, _) in &chapter_files {
        if scene_files.contains_key(ch_num) {
            chapters_with_scenes += 1;
        }
        
        if content_files.contains_key(ch_num) {
            chapters_with_content += 1;
        }
    }
    
    tracing::info!(
        "Book completion stats: {} chapters found, {} with scenes, {} with content (expecting {} chapters)",
        chapter_files.len(),
        chapters_with_scenes,
        chapters_with_content,
        expected_chapter_count
    );
    
    // A book is considered complete if:
    // 1. The number of chapters with content is close to or exceeds the expected count
    // 2. Most chapters have both scenes and content
    let chapter_threshold = (expected_chapter_count as f32 * 0.8).ceil() as usize;
    let is_complete = chapters_with_content >= chapter_threshold && 
                      chapters_with_scenes >= chapter_threshold;
    
    if is_complete {
        tracing::info!("Book considered complete based on comprehensive file analysis");
    }
    
    is_complete
}

/// Creates a flag file with the current timestamp to mark completion of a process.
pub fn create_flag_file(flag_path: &Path) -> io::Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut file = fs::File::create(flag_path)?;
    file.write_all(timestamp.as_bytes())?;
    Ok(())
}

/// Sanitizes a name to make it safe for use in directory names
pub fn sanitize_directory_name(name: &str) -> String {
    name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

/// Safely reads a metadata file from the output directory
pub fn read_metadata_file_safe(output_dir: &Path) -> Option<String> {
    let metadata_path = output_dir.join("metadata.md");
    
    // Check if file exists
    if !metadata_path.exists() {
        return None;
    }
    
    // Attempt to read the file
    match fs::read_to_string(&metadata_path) {
        Ok(content) => Some(content),
        Err(e) => {
            tracing::warn!("Failed to read metadata file: {}", e);
            None
        }
    }
}

/// Safely reads a section from the metadata file
pub fn read_metadata_section_safe(output_dir: &Path, section: &str) -> Option<String> {
    // Get the metadata content
    let content = read_metadata_file_safe(output_dir)?;
    
    // Find the section marker - try both with and without timestamp
    let section_marker = format!("\n## {}", section);
    let section_start = content.find(&section_marker)
        .or_else(|| {
            // Try to find section with timestamp in parentheses
            // This handles formats like "## Book Outline (2025-02-26 07:07:22)"
            content.find(&format!("\n## {} (", section))
        });
    
    if let Some(start) = section_start {
        // Find the end of this section (start of next section or end of file)
        let content_after_marker = &content[start + section_marker.len()..];
        let next_section = content_after_marker.find("\n## ");
        
        let section_content = if let Some(end) = next_section {
            &content_after_marker[..end]
        } else {
            content_after_marker
        };
        
        // Trim and return
        Some(section_content.trim().to_string())
    } else {
        None
    }
}

/// Updates the metadata file with new content for a section
pub fn update_metadata(output_dir: &Path, section: &str, content: &str) -> io::Result<()> {
    let metadata_path = output_dir.join("metadata.md");
    let mut metadata_content = if metadata_path.exists() {
        fs::read_to_string(&metadata_path)?
    } else {
        String::new()
    };

    // Add timestamp to the section
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let section_with_timestamp = format!("{} ({})", section, timestamp);

    let section_start = metadata_content.find(&format!("\n## {}", section));
    if let Some(start) = section_start {
        let section_end = metadata_content[start + 1..].find("\n## ").unwrap_or(metadata_content.len() - start - 1);
        metadata_content.replace_range(start..start + section_end + 1, &format!("\n## {}\n{}\n", section_with_timestamp, content));
    } else {
        metadata_content.push_str(&format!("\n## {}\n{}\n", section_with_timestamp, content));
    }

    fs::write(&metadata_path, metadata_content)?;

    // Update metadata.json with timestamp
    update_metadata_json(output_dir, &section_with_timestamp, content)?;

    Ok(())
}

/// Updates the metadata JSON file with new content
pub fn update_metadata_json(output_dir: &Path, key: &str, value: &str) -> io::Result<()> {
    let metadata_path = output_dir.join("metadata.json");
    let mut metadata = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::Map::new())
    } else {
        serde_json::Map::new()
    };

    metadata.insert(key.to_string(), serde_json::Value::String(value.to_string()));

    let json_string = serde_json::to_string_pretty(&metadata)?;
    fs::write(metadata_path, json_string)
}

/// Alias for read_metadata_section_safe for backward compatibility
pub fn read_metadata_section(output_dir: &Path, section: &str) -> Option<String> {
    read_metadata_section_safe(output_dir, section)
}

/// Helper function to sanitize filenames for PDF/EPUB generation
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Extracts chapter number from a filename
fn extract_chapter_number(filename: &str) -> Option<usize> {
    // Try different patterns for chapter numbers
    
    // Pattern: chapter_1, chapter_1_scene_2, etc.
    if let Some(pos) = filename.find("chapter_") {
        let start = pos + "chapter_".len();
        let end = filename[start..].find(|c: char| !c.is_ascii_digit())
            .map(|e| start + e)
            .unwrap_or(filename.len());
            
        if start < end {
            if let Ok(num) = filename[start..end].parse::<usize>() {
                return Some(num);
            }
        }
    }
    
    // Pattern: ch1, ch1_scene2, etc.
    if let Some(pos) = filename.find("ch") {
        let start = pos + "ch".len();
        let end = filename[start..].find(|c: char| !c.is_ascii_digit())
            .map(|e| start + e)
            .unwrap_or(filename.len());
            
        if start < end {
            if let Ok(num) = filename[start..end].parse::<usize>() {
                return Some(num);
            }
        }
    }
    
    None
} 