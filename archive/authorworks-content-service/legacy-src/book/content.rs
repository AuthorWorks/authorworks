#![allow(clippy::too_many_arguments)]

use crate::book::Context;
use crate::config::Config;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Content {
    pub text: String,
    pub chapter_number: usize,
    pub scene_number: usize,
}

impl Content {
    pub async fn generate(
        _context: &mut Context,
        config: &Config,
        chapter_number: usize,
        scene_number: usize,
        chapter_title: &str,
        scene_title: &str,
        scene_description: &str,
        chapter_description: &str,
        output_dir: &Path,
        token_tracker: &crate::utils::logging::TokenTracker,
    ) -> Result<Self> {
        let logs_dir = output_dir.join("logs");
        std::fs::create_dir_all(&logs_dir)?;

        let output_file = logs_dir.join(format!(
            "content_generation_ch{}_scene{}.txt",
            chapter_number, scene_number
        ));
        
        // Check if content already exists
        if output_file.exists() {
            let content = std::fs::read_to_string(&output_file)?;
            if !content.trim().is_empty() {
                println!("    📋 Using existing content for scene {} in chapter {}", scene_number, chapter_number);
                return Ok(Self {
                    text: content,
                    chapter_number,
                    scene_number,
                });
            }
        }

        // Create the prompt for content generation
        let mut prompt = String::new();
        prompt.push_str(&format!("# Content Generation for Chapter {} Scene {}\n\n", chapter_number, scene_number));
        prompt.push_str(&format!("## Chapter Title: {}\n", chapter_title));
        prompt.push_str(&format!("## Chapter Description: {}\n\n", chapter_description));
        prompt.push_str(&format!("## Scene Title: {}\n", scene_title));
        prompt.push_str(&format!("## Scene Description: {}\n\n", scene_description));
        
        prompt.push_str("## Instructions\n");
        prompt.push_str("Write the full content for this scene based on the description provided. ");
        prompt.push_str("The content should be engaging, descriptive, and match the tone of the book. ");
        prompt.push_str("Include dialogue, action, and description as appropriate for the scene. ");
        prompt.push_str("The content should be at least 500 words but can be longer if needed to fully develop the scene.\n\n");
        
        prompt.push_str("## Scene Content\n");

        // Log the prompt
        let prompt_file = logs_dir.join(format!(
            "content_generation_ch{}_scene{}_prompt.txt",
            chapter_number, scene_number
        ));
        std::fs::write(&prompt_file, &prompt)?;

        // Generate the content
        let model = config.get_model_for_phase("content");
        let content = crate::llm::generate(model, &prompt, token_tracker).await?;

        // Save the content
        std::fs::write(&output_file, &content)?;
        
        // Also save as markdown for easier reading
        let md_file = logs_dir.join(format!(
            "chapter_{}_scene_{}_content.md",
            chapter_number, scene_number
        ));
        std::fs::write(&md_file, &content)?;

        Ok(Self {
            text: content,
            chapter_number,
            scene_number,
        })
    }
}