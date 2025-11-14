use std::path::Path;
use crate::book::Book;
use crate::error::Result;
use std::fs;
use std::io::Write;
use std::fmt;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BookStatistics {
    pub total_characters: usize,
    pub total_words: usize,
    pub total_chapters: usize,
    pub total_scenes: usize,
    pub avg_words_per_chapter: usize,
    pub avg_words_per_scene: usize,
    pub estimated_pages: usize,
    pub reading_time_minutes: usize,
    pub total_generation_time: Duration,
    pub phase_timings: HashMap<String, Duration>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_cost: f64,
}

impl BookStatistics {
    pub fn new(book: &Book) -> Self {
        let mut total_characters = 0;
        let mut total_words = 0;
        let total_chapters = book.chapters.len();
        let mut total_scenes = 0;
        
        // Calculate statistics
        for chapter in &book.chapters {
            total_scenes += chapter.scenes.len();
            
            for scene in &chapter.scenes {
                let text = &scene.content.text;
                total_characters += text.chars().count();
                total_words += text.split_whitespace().count();
            }
        }
        
        // Calculate averages and estimates
        let avg_words_per_chapter = if total_chapters > 0 {
            total_words / total_chapters
        } else {
            0
        };
        
        let avg_words_per_scene = if total_scenes > 0 {
            total_words / total_scenes
        } else {
            0
        };
        
        // Industry standard: ~250-300 words per page for a novel
        let words_per_page = 275;
        let estimated_pages = (total_words as f64 / words_per_page as f64).ceil() as usize;
        
        // Average reading speed: ~250 words per minute
        let words_per_minute = 250;
        let reading_time_minutes = (total_words as f64 / words_per_minute as f64).ceil() as usize;
        
        Self {
            total_characters,
            total_words,
            total_chapters,
            total_scenes,
            avg_words_per_chapter,
            avg_words_per_scene,
            estimated_pages,
            reading_time_minutes,
            total_generation_time: Duration::from_secs(0),
            phase_timings: HashMap::new(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_cost: 0.0,
        }
    }
    
    pub fn with_timings(book: &Book, total_time: Duration, phase_timings: HashMap<String, Duration>) -> Self {
        let mut stats = Self::new(book);
        stats.total_generation_time = total_time;
        stats.phase_timings = phase_timings;
        stats
    }
    
    pub fn with_tokens(book: &Book, token_tracker: &crate::utils::logging::TokenTracker) -> Self {
        let mut stats = Self::new(book);
        stats.prompt_tokens = token_tracker.get_prompt_tokens() as u64;
        stats.completion_tokens = token_tracker.get_completion_tokens() as u64;
        
        // Calculate cost using TokenUsageStats formula
        // Constants from logging.rs
        const CLAUDE_INPUT_PRICE: f64 = 3.0;  // $3 per 1M input tokens
        const CLAUDE_OUTPUT_PRICE: f64 = 15.0; // $15 per 1M output tokens
        
        let prompt_cost = (stats.prompt_tokens as f64 / 1_000_000.0) * CLAUDE_INPUT_PRICE;
        let completion_cost = (stats.completion_tokens as f64 / 1_000_000.0) * CLAUDE_OUTPUT_PRICE;
        stats.total_cost = prompt_cost + completion_cost;
        
        stats
    }
    
    pub fn with_timings_and_tokens(
        book: &Book, 
        total_time: Duration, 
        phase_timings: HashMap<String, Duration>,
        token_tracker: &crate::utils::logging::TokenTracker
    ) -> Self {
        let mut stats = Self::with_timings(book, total_time, phase_timings);
        stats.prompt_tokens = token_tracker.get_prompt_tokens() as u64;
        stats.completion_tokens = token_tracker.get_completion_tokens() as u64;
        
        // Calculate cost using TokenUsageStats formula
        const CLAUDE_INPUT_PRICE: f64 = 3.0;  // $3 per 1M input tokens
        const CLAUDE_OUTPUT_PRICE: f64 = 15.0; // $15 per 1M output tokens
        
        let prompt_cost = (stats.prompt_tokens as f64 / 1_000_000.0) * CLAUDE_INPUT_PRICE;
        let completion_cost = (stats.completion_tokens as f64 / 1_000_000.0) * CLAUDE_OUTPUT_PRICE;
        stats.total_cost = prompt_cost + completion_cost;
        
        stats
    }
    
    pub fn save_to_file(&self, output_dir: &Path) -> Result<()> {
        let stats_path = output_dir.join("book_statistics.txt");
        let mut file = fs::File::create(stats_path)?;
        writeln!(file, "{}", self)?;
        Ok(())
    }
    
    pub fn update_metadata(&self, output_dir: &Path) -> std::io::Result<()> {
        crate::update_metadata(
            output_dir,
            "Book Statistics",
            &format!("{}", self)
        )
    }
    
    fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

impl fmt::Display for BookStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "📊 Book Statistics:\n\
             - Total Words: {}\n\
             - Total Characters: {}\n\
             - Chapters: {}\n\
             - Scenes: {}\n\
             - Average Words per Chapter: {}\n\
             - Average Words per Scene: {}\n\
             - Estimated Pages: {}\n\
             - Estimated Reading Time: {} hours {} minutes",
            self.total_words,
            self.total_characters,
            self.total_chapters,
            self.total_scenes,
            self.avg_words_per_chapter,
            self.avg_words_per_scene,
            self.estimated_pages,
            self.reading_time_minutes / 60,
            self.reading_time_minutes % 60
        )?;
        
        // Add token usage and cost information if available
        if self.prompt_tokens > 0 || self.completion_tokens > 0 {
            let total_tokens = self.prompt_tokens + self.completion_tokens;
            write!(
                f,
                "\n\n🔤 Token Usage:\n\
                 - Prompt Tokens: {}\n\
                 - Completion Tokens: {}\n\
                 - Total Tokens: {}\n\
                 - Estimated Cost: ${:.2}",
                self.prompt_tokens,
                self.completion_tokens,
                total_tokens,
                self.total_cost
            )?;
        }
        
        if self.total_generation_time.as_secs() > 0 {
            write!(
                f,
                "\n\n⏱️ Generation Time Statistics:\n\
                 - Total Generation Time: {}",
                Self::format_duration(self.total_generation_time)
            )?;
            
            if !self.phase_timings.is_empty() {
                writeln!(f, "\n- Phase Timings:")?;
                
                // Define the order of phases for consistent output
                let phase_order = [
                    "Phase 1: Initial Setup and Context",
                    "Phase 2: Book Structure",
                    "Phase 3: Chapter Generation",
                    "Phase 4: Scene Generation",
                    "Phase 5: Content Generation"
                ];
                
                // Display phases in the defined order
                for phase_name in &phase_order {
                    if let Some(duration) = self.phase_timings.get(*phase_name) {
                        writeln!(f, "  • {}: {}", phase_name, Self::format_duration(*duration))?;
                    }
                }
                
                // Display any additional phases not in the predefined order
                let mut other_phases: Vec<(&String, &Duration)> = self.phase_timings.iter()
                    .filter(|(name, _)| !phase_order.contains(&name.as_str()))
                    .collect();
                
                if !other_phases.is_empty() {
                    other_phases.sort_by(|a, b| a.0.cmp(b.0));
                    
                    for (phase, duration) in other_phases {
                        writeln!(f, "  • {}: {}", phase, Self::format_duration(*duration))?;
                    }
                }
            }
        }
        
        Ok(())
    }
} 