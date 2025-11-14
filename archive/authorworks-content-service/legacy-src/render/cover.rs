use std::path::{Path, PathBuf};
use crate::book::Book;
use crate::error::{Result, BookGeneratorError};
use std::fs;
use reqwest;
use serde_json::{json, Value};
use std::env;
use tracing::{info, warn};
use crate::utils::prompts::Prompts;
use langchain_rust::prompt::PromptFromatter;
use crate::llm;
use crate::utils::logging::TokenTracker;

/// Generates a cover image for a book using AI
pub async fn generate_cover_image(book: &Book, output_dir: &Path) -> Result<PathBuf> {
    info!("Generating cover image for book: {}", book.context.title);
    
    // Create the prompt for the cover image using the Prompts module
    let prompt_template = Prompts::cover_image();
    let prompt = prompt_template.format(langchain_rust::prompt_args!{
        "title" => book.context.title.clone(),
        "genre" => book.context.genre.name.clone(),
        "braindump" => book.context.braindump.content.clone(),
        "style" => book.context.style.description.clone(),
        "synopsis" => book.context.synopsis.content.clone(),
        "book_outline" => book.context.outline.to_string(),
    })?;
    
    // Log the prompt
    crate::utils::logging::log_prompt(
        output_dir,
        "cover_image_generation_prompt",
        &prompt,
        "Cover image generation",
    )?;
    
    // Generate the image using the generic image generation function
    let image_path = generate_image(&prompt, output_dir, "cover").await?;
    
    info!("Cover image generated successfully at: {:?}", image_path);
    
    Ok(image_path)
}

/// Generic function to generate an image using the configured LLM
pub async fn generate_image(prompt: &str, output_dir: &Path, filename: &str) -> Result<PathBuf> {
    // Create a token tracker for this operation
    let token_tracker = TokenTracker::new();
    
    // Create the LLM client
    let client = llm::create_client()
        .map_err(|e| BookGeneratorError::Other(format!("Failed to create LLM client: {}", e)))?;
    
    // Use the configured LLM to generate SVG image based on the prompt
    // Use the default model configured in the system
    let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-3-opus-20240229".to_string());
    
    // Set a more reasonable max_tokens value for image generation
    let response = client.generate_with_options(&model, prompt, Some(4096))
        .await
        .map_err(|e| BookGeneratorError::Other(format!("Failed to generate image: {}", e)))?;
    
    // Track token usage
    if let Some(usage) = &response.usage {
        token_tracker.add_prompt_tokens(usage.prompt_tokens);
        token_tracker.add_completion_tokens(usage.completion_tokens);
        
        // Log token usage
        crate::utils::logging::log_tokens(
            output_dir,
            &format!("{} image generation", filename),
            usage.prompt_tokens as u32,
            usage.completion_tokens as u32,
        ).map_err(|e| BookGeneratorError::Other(format!("Failed to log tokens: {}", e)))?;
    }
    
    // Extract SVG content from the response
    let svg_content = extract_svg_from_response(&response.text)
        .ok_or_else(|| BookGeneratorError::Other("No SVG content found in LLM response".to_string()))?;
    
    // Create the images directory if it doesn't exist
    let images_dir = output_dir.join("images");
    fs::create_dir_all(&images_dir)
        .map_err(|e| BookGeneratorError::Other(format!("Failed to create images directory: {}", e)))?;
    
    // Save as both SVG and PNG for compatibility
    let svg_path = images_dir.join(format!("{}.svg", filename));
    fs::write(&svg_path, svg_content)
        .map_err(|e| BookGeneratorError::Other(format!("Failed to write SVG file: {}", e)))?;
    
    // Also create a PNG version for better compatibility with some readers
    let png_path = images_dir.join(format!("{}.png", filename));
    match convert_svg_to_png(&svg_path, &png_path) {
        Ok(_) => {
            info!("Successfully created both SVG and PNG versions of the image");
            // Return the PNG path if it was successfully created, as it's more widely compatible
            if png_path.exists() {
                return Ok(png_path);
            }
        },
        Err(e) => {
            warn!("Failed to convert SVG to PNG: {}. Using SVG only.", e);
        }
    }
    
    // Return the SVG path as fallback
    Ok(svg_path)
}

/// Convert SVG to PNG for better compatibility
/// 
/// This conversion is useful because:
/// 1. Some e-readers have limited SVG support
/// 2. PNG is more widely supported across different platforms
/// 3. Some publishing platforms require raster images
/// 
/// However, SVG is preferred for quality when supported, so we keep both formats.
fn convert_svg_to_png(svg_path: &Path, png_path: &Path) -> Result<()> {
    info!("Converting SVG to PNG for better compatibility: {:?} -> {:?}", svg_path, png_path);
    
    // Try to use librsvg or other tools if available
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // Try using rsvg-convert if available
        info!("Attempting conversion with rsvg-convert...");
        let status = Command::new("rsvg-convert")
            .arg("-o")
            .arg(png_path)
            .arg(svg_path)
            .status();
            
        if let Ok(status) = status {
            if status.success() {
                info!("Successfully converted SVG to PNG using rsvg-convert");
                return Ok(());
            } else {
                warn!("rsvg-convert failed with status: {}", status);
            }
        } else {
            warn!("rsvg-convert not found or failed to execute");
        }
        
        // Try using other tools if rsvg-convert fails
        info!("Attempting conversion with ImageMagick convert...");
        let status = Command::new("convert")  // ImageMagick
            .arg(svg_path)
            .arg(png_path)
            .status();
            
        if let Ok(status) = status {
            if status.success() {
                info!("Successfully converted SVG to PNG using ImageMagick");
                return Ok(());
            } else {
                warn!("ImageMagick convert failed with status: {}", status);
            }
        } else {
            warn!("ImageMagick convert not found or failed to execute");
        }
        
        // Try using Inkscape as a last resort
        info!("Attempting conversion with Inkscape...");
        let status = Command::new("inkscape")
            .arg("--export-filename")
            .arg(png_path)
            .arg(svg_path)
            .status();
            
        if let Ok(status) = status {
            if status.success() {
                info!("Successfully converted SVG to PNG using Inkscape");
                return Ok(());
            } else {
                warn!("Inkscape failed with status: {}", status);
            }
        } else {
            warn!("Inkscape not found or failed to execute");
        }
        
        // Log that conversion failed but continue with SVG only
        warn!("Could not convert SVG to PNG. Some e-readers may not display the cover correctly.");
    }
    
    // On other platforms or if conversion fails, just continue with SVG
    #[cfg(not(target_os = "macos"))]
    {
        warn!("SVG to PNG conversion not implemented for this platform. Using SVG only.");
    }
    
    // Return error to indicate conversion failed
    Err(BookGeneratorError::Other("Failed to convert SVG to PNG. No suitable conversion tool found.".to_string()))
}

/// Extract SVG content from LLM response
fn extract_svg_from_response(response: &str) -> Option<String> {
    // Look for SVG content between <svg> and </svg> tags
    if let Some(start) = response.find("<svg") {
        if let Some(end) = response[start..].find("</svg>") {
            return Some(response[start..start + end + 6].to_string());
        }
    }
    
    // Alternative: look for SVG content between ```svg and ``` code blocks
    if let Some(start) = response.find("```svg") {
        let content_start = start + 6;
        if let Some(end) = response[content_start..].find("```") {
            return Some(response[content_start..content_start + end].trim().to_string());
        }
    }
    
    // If no SVG found, return None
    None
}

/// Fallback function to generate a simple SVG cover if API generation fails
pub fn generate_fallback_cover(book: &Book, output_dir: &Path) -> Result<PathBuf> {
    info!("Generating fallback SVG cover for book: {}", book.context.title);
    
    // Create a simple SVG with the book title
    let title = &book.context.title;
    let genre = &book.context.genre.name;
    
    // Generate random colors based on the book title and genre
    let color1 = generate_color_from_text(title);
    let color2 = generate_color_from_text(genre);
    
    // Create a simple SVG with gradients
    let svg_content = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">
        <defs>
            <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" style="stop-color:{};stop-opacity:1" />
                <stop offset="100%" style="stop-color:{};stop-opacity:1" />
            </linearGradient>
        </defs>
        <rect width="1024" height="1024" fill="url(#grad1)" />
        <text x="512" y="480" font-family="Arial" font-size="60" text-anchor="middle" fill="white">{}</text>
        <text x="512" y="560" font-family="Arial" font-size="40" text-anchor="middle" fill="white">A Novel</text>
    </svg>"#, color1, color2, title);
    
    // Create the images directory if it doesn't exist
    let images_dir = output_dir.join("images");
    fs::create_dir_all(&images_dir)
        .map_err(|e| BookGeneratorError::Other(format!("Failed to create images directory: {}", e)))?;
    
    // Save the SVG
    let svg_path = images_dir.join("cover.svg");
    fs::write(&svg_path, svg_content)
        .map_err(|e| BookGeneratorError::Other(format!("Failed to write SVG file: {}", e)))?;
    
    // Also save as PNG for better compatibility
    let png_path = images_dir.join("cover.png");
    convert_svg_to_png(&svg_path, &png_path)?;
    
    Ok(svg_path)
}

/// Generates a color from text
fn generate_color_from_text(text: &str) -> String {
    let hash = text.chars().fold(0, |acc, c| acc + c as u32);
    let r = (hash % 200) + 55; // Avoid too dark colors
    let g = ((hash / 256) % 200) + 55;
    let b = ((hash / 65536) % 200) + 55;
    format!("rgb({}, {}, {})", r, g, b)
} 