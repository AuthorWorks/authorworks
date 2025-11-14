/// Convert a string to title case
pub fn title_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c.is_alphabetic() {
            if capitalize_next {
                result.push(c.to_uppercase().next().unwrap());
                capitalize_next = false;
            } else {
                result.push(c.to_lowercase().next().unwrap());
            }
        } else {
            result.push(c);
            if c.is_whitespace() || c == '-' || c == ':' {
                capitalize_next = true;
            }
        }
    }
    
    result
}

/// Helper function to clean chapter titles to ensure consistency
pub fn clean_chapter_title(title: &str) -> String {
    // If the title contains a book title (which might be in the format of the directory name),
    // we should try to detect and remove it generically rather than hardcoding a specific book name
    let title = remove_book_title_from_chapter(title);
    
    // Remove "- Detailed Outline" suffix if present
    let title = if title.ends_with("- Detailed Outline") {
        title.trim_end_matches("- Detailed Outline").trim().to_string()
    } else {
        title.to_string()
    };
    
    // If title is in format "Chapter X: Chapter Y", extract just the unique part
    if title.contains("Chapter") && title.contains(":") {
        let parts: Vec<&str> = title.split(":").collect();
        if parts.len() > 1 {
            let chapter_num_part = parts[0].trim();
            let chapter_title_part = parts[1].trim();
            
            // If the title part is just repeating the chapter number or is empty, use a generic title
            if chapter_title_part.starts_with("Chapter ") || 
               chapter_title_part == chapter_num_part || 
               chapter_title_part.trim().is_empty() {
                // Extract the chapter number
                if let Ok(num_str) = chapter_num_part.trim_start_matches("Chapter ").trim().parse::<usize>() {
                    // Try to find a title in any available outline file
                    if let Some(outline_title) = find_chapter_title_in_outlines(num_str) {
                        return outline_title;
                    }
                    
                    return format!("Chapter {}", num_str);
                } else {
                    // Just return a clean version of the chapter number
                    return chapter_num_part.trim_start_matches("Chapter ").trim().to_string();
                }
            }
            
            // Otherwise return the meaningful part after the colon
            return chapter_title_part.to_string();
        }
    }
    
    // Handle uppercase titles like "CHAPTER 4: THE DISGRACED SCHOLAR"
    if title.to_uppercase() == title && title.contains("CHAPTER") {
        // Convert to title case instead of all caps
        return title_case(&title);
    }
    
    // If we have a raw chapter number, try to find the title in the outline
    if title.starts_with("Chapter ") {
        if let Ok(num_str) = title.trim_start_matches("Chapter ").trim().parse::<usize>() {
            if let Some(outline_title) = find_chapter_title_in_outlines(num_str) {
                return outline_title;
            }
        }
    }
    
    title
}

/// Helper function to remove book title from chapter title
fn remove_book_title_from_chapter(title: &str) -> String {
    // Try common output directory locations
    let possible_output_dirs = ["output", "."];
    
    for output_dir in possible_output_dirs.iter() {
        if let Ok(entries) = std::fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(dir_name) = entry.path().file_name() {
                        let book_name = dir_name.to_string_lossy().to_lowercase();
                        
                        // Skip very short book names (less than 3 chars) to avoid false positives
                        if book_name.len() < 3 {
                            continue;
                        }
                        
                        // Check if the book name is in the title (as a distinct word or phrase)
                        let title_lower = title.to_lowercase();
                        if title_lower.contains(&book_name) {
                            // Make sure it's not just a substring match (check for word boundaries)
                            // This helps avoid removing parts of words that happen to contain the book name
                            let is_distinct_match = 
                                title_lower.starts_with(&book_name) || 
                                title_lower.contains(&format!(" {} ", book_name)) ||
                                title_lower.ends_with(&book_name) ||
                                title_lower.contains(&format!("{} -", book_name)) ||
                                title_lower.contains(&format!("- {}", book_name));
                                
                            if is_distinct_match {
                                // Remove the book name (both lowercase and uppercase versions)
                                let mut title_without_book = title.to_string();
                                
                                // Remove the book name with various formatting
                                let patterns = [
                                    book_name.clone(),
                                    book_name.to_uppercase(),
                                    format!("{} - ", book_name),
                                    format!("{} - ", book_name.to_uppercase()),
                                    format!("- {} ", book_name),
                                    format!("- {} ", book_name.to_uppercase()),
                                ];
                                
                                for pattern in patterns.iter() {
                                    title_without_book = title_without_book.replace(pattern, "");
                                }
                                
                                // Clean up any remaining artifacts
                                let result = title_without_book.trim().to_string();
                                
                                // If we've removed too much, keep the original
                                if !result.is_empty() {
                                    return result;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // If we couldn't find a matching book title, return the original
    title.to_string()
}

/// Helper function to find chapter titles in outline files
fn find_chapter_title_in_outlines(chapter_num: usize) -> Option<String> {
    // Try to find outline files in common locations
    let possible_paths = [
        // Generic paths that work for any book
        "output/raw_outline_output.txt",
        "raw_outline_output.txt",
    ];
    
    for path in possible_paths.iter() {
        if let Ok(outline) = std::fs::read_to_string(path) {
            for line in outline.lines() {
                if line.starts_with(&format!("Chapter {}: ", chapter_num)) {
                    let outline_title = line.split(":").nth(1).unwrap_or("").trim();
                    if !outline_title.is_empty() {
                        return Some(outline_title.to_string());
                    }
                }
            }
        }
    }
    
    // If we couldn't find the outline file, check the output directory
    if let Ok(entries) = std::fs::read_dir("output") {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let outline_path = entry.path().join("raw_outline_output.txt");
                if let Ok(outline) = std::fs::read_to_string(&outline_path) {
                    for line in outline.lines() {
                        if line.starts_with(&format!("Chapter {}: ", chapter_num)) {
                            let outline_title = line.split(":").nth(1).unwrap_or("").trim();
                            if !outline_title.is_empty() {
                                return Some(outline_title.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
} 