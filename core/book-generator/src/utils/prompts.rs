use langchain_rust::prompt::{PromptTemplate, TemplateFormat};

pub struct Prompts;

impl Prompts {
    pub fn braindump() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a braindump for a novel titled '{title}', include key ideas, themes, and potential plot points.\n\nBraindump:".to_string(),
            vec!["title".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn genre() -> PromptTemplate {
        PromptTemplate::new(
            "Given the following context, suggest an appropriate genre for the book. Provide the genre name followed by a colon and a brief description of why this genre fits the book.\n\nTitle: {title}\nBraindump: {braindump}\n\nGenre:".to_string(),
            vec!["title".to_string(), "braindump".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn style() -> PromptTemplate {
        PromptTemplate::new(
            "Based on the following context, suggest a writing style for the novel. Describe the narrative perspective, tense, and any notable stylistic elements.\n\nTitle: {title}\nBraindump: {braindump}\nGenre: {genre}\n\nStyle:".to_string(),
            vec!["title".to_string(), "braindump".to_string(), "genre".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn characters() -> PromptTemplate {
        PromptTemplate::new(
            "Create a list of characters based on the following context. For each character, provide a name followed by a colon and a brief description that fits the context, genre, and style. Format each character on a new line.\n\nTitle: {title}\nBraindump: {braindump}\nGenre: {genre}\nStyle: {style}\n\nCharacters:".to_string(),
            vec!["title".to_string(), "braindump".to_string(), "genre".to_string(), "style".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn synopsis() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a synopsis based on the following context. Provide concise but thorough summaries of the main plot, themes, and character arcs. The synopsis should reflect the genre, style, and overall tone of the book as described in the context. Focus on the core narrative and central conflicts while touching on the key thematic elements. Do not include placeholders or instructions in your response. \n\nTitle: {title}\nBraindump: {braindump}\nGenre: {genre}\nStyle: {style}\nCharacters: {characters}\n\nSynopsis:".to_string(),
            vec!["title".to_string(), "braindump".to_string(), "genre".to_string(), "style".to_string(), "characters".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn outline() -> PromptTemplate {
        PromptTemplate::new(
            "Based on the following context, generate a comprehensive chapter-by-chapter outline. Each chapter should have a natural number of scenes based on its narrative needs - some chapters may need only 1-2 scenes to tell their story effectively, while others may require 3-5 scenes or more to properly develop their dramatic arc. Consider pacing, dramatic tension, and narrative flow when determining the number of scenes per chapter.\n\nTitle: {title}\nBraindump: {braindump}\nGenre: {genre}\nStyle: {style}\nCharacters: {characters}\nSynopsis: {synopsis}\n\nIMPORTANT: You MUST follow this EXACT format for the outline:\n\nChapter 1: [Title]\n[Chapter Description]\nScene 1: [Title]\n[Scene Description]\nScene 2: [Title]\n[Scene Description]\n\nChapter 2: [Title]\n[Chapter Description]\n[And so on...]\n\nEach chapter MUST start with the word 'Chapter' followed by a number and a colon (e.g., 'Chapter 1: Title'). Make sure to include at least 10 chapters in your outline.\n\nDo NOT include any additional formatting, headers, or explanatory text. Start directly with 'Chapter 1:' and end with the last scene description.\n\nOutline:".to_string(),
            vec!["title".to_string(), "braindump".to_string(), "genre".to_string(), "style".to_string(), "characters".to_string(), "synopsis".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn chapter() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a detailed chapter outline based on the following context maintaining narrative progression and story continuity.\n\nTitle: {title}\nBraindump: {braindump}\nGenre: {genre}\nStyle: {style}\nCharacters: {characters}\nSynopsis: {synopsis}\nBook Outline: {book_outline}\nTemporary Summary: {temporary_summary}\n\nChapter Outline: Create a chapter outline that continues the story naturally from the previous context while advancing the plot according to the book outline. Focus on character development, thematic elements, and maintaining the established tone.".to_string(),
            vec!["title".to_string(), "braindump".to_string(), "genre".to_string(), "style".to_string(), "characters".to_string(), "synopsis".to_string(), "book_outline".to_string(), "temporary_summary".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn scene() -> PromptTemplate {
        PromptTemplate::new(
            "Create a scene outline using the following context, consider character interactions, setting details, and emotional resonance. The scene should naturally follow from previous scenes while advancing the chapter's narrative.\n\nBook Synopsis: {synopsis}\nChapter Outline: {chapter_outline}\nPrevious Scenes: {previous_scenes}\nTemporary Summary: {temporary_summary}\n\nScene Outline:".to_string(),
            vec!["synopsis".to_string(), "chapter_outline".to_string(), "previous_scenes".to_string(), "temporary_summary".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn temporary_summary_chapter() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a concise but comprehensive summary of all previous context to maintain story continuity. This summary should incorporate the title, braindump, genre, style, characters, synopsis, book outline, and all previously generated chapter outlines. The summary should focus on maintaining narrative flow and ensuring all context is properly integrated.\n\nContext:\n{context}\n\nProvide a focused summary that captures all context and story progression so far, ending with 'Here is where we continue the story...':\n".to_string(),
            vec!["context".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn temporary_summary_scene() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a scene-focused summary incorporating all previous context to maintain story continuity. This summary should include:\n- Title, braindump, genre, style\n- Book synopsis\n- Book outline\n- All chapter outlines up to and including the current chapter\n- All previously generated scenes from the current chapter\n\nFocus on the immediate context needed for the next scene while maintaining overall story coherence.\n\nContext:\n{context}\n\nProvide a focused summary that captures all context and scene progression so far, ending with 'Here is where we continue the story...':\n".to_string(),
            vec!["context".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn temporary_summary_content() -> PromptTemplate {
        PromptTemplate::new(
            "Generate a content-focused summary incorporating all previous context to maintain story continuity. This summary should include:\n- Title, genre, style\n- Book synopsis\n- Book outline\n- Current chapter outline\n- Current scene outline\n- All previously generated content from this chapter\n\nFocus on the immediate narrative flow while maintaining overall story coherence.\n\nContext:\n{context}\n\nProvide a focused summary that captures all context and content progression so far, ending with 'Here is where we continue the story...':\n".to_string(),
            vec!["context".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn content() -> PromptTemplate {
        PromptTemplate::new(
            "Transform this scene outline into polished prose that captivates and resonates with readers. The writing should be realistic, whimsical, creative, clever, occasionally wryly humorous, and wise - most importantly, it should demonstrate deep understanding of human nature and masterful use of writing techniques.\n\nRemember this guidance on prose rhythm: 'This sentence has five words. Here are five more words. Five-word sentences are fine. But several together become monotonous. Listen to what is happening. The writing is getting boring. The sound of it drones. It's like a stuck record. The ear demands some variety. Now listen. I vary the sentence length, and I create music. Music. The writing sings. It has a pleasant rhythm, a lilt, a harmony. I use short sentences. And I use sentences of medium length. And sometimes, when I am certain the reader is rested, I will engage him with a sentence of considerable length, a sentence that burns with energy and builds with all the impetus of a crescendo, the roll of the drums, the crash of the cymbals—sounds that say listen to this, it is important.'\n\nStyle: {style}\nBook Synopsis: {synopsis}\nChapter Outline: {chapter_outline}\nScene Outline: {scene_outline}\nTemporary Summary: {temporary_summary}\nPrevious Content: {previous_content}\n\nWrite the complete scene:".to_string(),
            vec!["style".to_string(), "synopsis".to_string(), "chapter_outline".to_string(), "scene_outline".to_string(), "temporary_summary".to_string(), "previous_content".to_string()],
            TemplateFormat::FString,
        )
    }

    pub fn cover_image() -> PromptTemplate {
        PromptTemplate::new(
            "Create a minimalist geometric SVG composition that abstractly captures the essence of \"{title}\", using precise mathematical patterns (circles, waves, or golden ratio arrangements) arranged in a visually harmonious grid structure. Incorporate 3-4 gradient colorways that reflect the emotional palette of this {genre} story, varying opacities to create depth, and strategic negative space to enhance visual rhythm. The design should feature subtle visual metaphors for key narrative elements—character arcs as intersecting paths, climactic moments as focal points, thematic tensions as balanced opposing forms—while maintaining a clean, contemporary aesthetic that stands alone as beautiful abstract art. Prioritize mathematical precision in placement, deliberate use of connecting lines or markers at pivotal points, and an overall composition that evokes the pacing and emotional resonance of the source material without being overtly representational.\n\nBook Context:\nTitle: {title}\nGenre: {genre}\nBraindump: {braindump}\nWriting Style: {style}\nSynopsis: {synopsis}\nBook Outline: {book_outline}".to_string(),
            vec!["title".to_string(), "genre".to_string(), "braindump".to_string(), "style".to_string(), "synopsis".to_string(), "book_outline".to_string()],
            TemplateFormat::FString,
        )
    }
}
