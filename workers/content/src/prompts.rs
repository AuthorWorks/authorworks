//! Prompt templates for AI content generation

pub fn build_outline_prompt(
    title: &str,
    description: &str,
    genre: &str,
    style: &str,
    chapter_count: i32,
    user_prompt: &str,
) -> String {
    format!(r#"Create a detailed book outline for the following project:

**Title:** {title}
**Genre:** {genre}
**Style:** {style}
**Description:** {description}

**Author's Notes:** {user_prompt}

Please create an outline with exactly {chapter_count} chapters. For each chapter, provide:
1. A compelling chapter title
2. A 2-3 paragraph outline of what happens
3. 3-5 key events or plot points

Also provide:
- A 2-3 paragraph synopsis of the entire book
- 3-5 major themes explored in the story

Format your response as follows:

## Synopsis
[Your synopsis here]

## Themes
- [Theme 1]
- [Theme 2]
- [Theme 3]

## Chapter 1: [Title]
[Chapter outline]
Key events:
- [Event 1]
- [Event 2]
- [Event 3]

## Chapter 2: [Title]
...and so on for all {chapter_count} chapters.

Make the outline engaging, with clear character development, rising tension, and satisfying resolution. Ensure each chapter has a clear purpose in advancing the plot or developing characters."#,
        title = title,
        genre = genre,
        style = style,
        description = description,
        user_prompt = user_prompt,
        chapter_count = chapter_count
    )
}

pub fn build_chapter_prompt(
    book_title: &str,
    chapter_title: &str,
    chapter_number: i32,
    outline: &str,
    context: &str,
    style: &str,
) -> String {
    format!(r#"Write Chapter {chapter_number} of "{book_title}".

**Chapter Title:** {chapter_title}

**Chapter Outline:**
{outline}

**Previous Context:**
{context}

**Writing Style:** {style}

Please write the complete chapter following these guidelines:
1. Write in third person limited or omniscient perspective (maintain consistency with any previous chapters)
2. Include vivid descriptions and sensory details
3. Write natural, character-appropriate dialogue
4. Show character emotions through actions and body language
5. Maintain appropriate pacing - balance action, dialogue, and description
6. End the chapter with a hook or transition to the next chapter
7. Target approximately 3,000-5,000 words

Write the full chapter text without meta-commentary. Begin directly with the chapter content."#,
        chapter_number = chapter_number,
        book_title = book_title,
        chapter_title = chapter_title,
        outline = if outline.is_empty() { "Write an engaging chapter that advances the story" } else { outline },
        context = if context.is_empty() { "This is the beginning of the book." } else { context },
        style = style
    )
}

pub fn build_enhancement_prompt(
    content: &str,
    enhancement_type: &str,
    instructions: Option<&str>,
) -> String {
    let type_instructions = match enhancement_type {
        "grammar" => "Fix any grammatical errors, punctuation mistakes, and awkward phrasing while preserving the author's voice.",
        "style" => "Enhance the prose style - improve word choice, vary sentence structure, and strengthen the overall flow.",
        "dialog" => "Improve dialogue to be more natural, distinctive to each character, and engaging. Add appropriate dialogue tags and beats.",
        "description" => "Enhance descriptive passages with more vivid imagery, sensory details, and atmospheric elements.",
        "pacing" => "Adjust the pacing - tighten slow sections, expand rushed moments, and ensure proper rhythm.",
        "continuity" => "Check for and fix any continuity issues, inconsistencies, or plot holes.",
        "all" => "Perform a comprehensive edit: fix grammar, enhance style, improve dialogue, strengthen descriptions, and adjust pacing.",
        _ => "Improve the overall quality of the writing."
    };

    let custom = instructions.map(|i| format!("\n\n**Additional Instructions:** {}", i)).unwrap_or_default();

    format!(r#"Please enhance the following content.

**Enhancement Type:** {enhancement_type}
**Instructions:** {type_instructions}{custom}

**Original Content:**
{content}

**Guidelines:**
1. Maintain the author's voice and intent
2. Keep all plot points and character actions intact
3. Preserve the original meaning
4. Only make improvements, not fundamental changes
5. Return the complete enhanced text

Provide the enhanced version of the content:"#,
        enhancement_type = enhancement_type,
        type_instructions = type_instructions,
        custom = custom,
        content = content
    )
}

pub fn build_synopsis_prompt(
    title: &str,
    genre: &str,
    description: &str,
    chapter_summaries: &str,
) -> String {
    format!(r#"Write a compelling book synopsis for:

**Title:** {title}
**Genre:** {genre}
**Description:** {description}

**Chapter Summaries:**
{chapter_summaries}

Create a 2-3 paragraph synopsis that:
1. Hooks the reader immediately
2. Introduces the main character(s) and their goals
3. Presents the central conflict without spoiling the ending
4. Conveys the tone and atmosphere of the book
5. Is suitable for marketing/back cover use

Write the synopsis in present tense, third person."#,
        title = title,
        genre = genre,
        description = description,
        chapter_summaries = chapter_summaries
    )
}

pub fn build_character_prompt(
    character_name: &str,
    role: &str,
    existing_details: &str,
    book_context: &str,
) -> String {
    format!(r#"Develop a detailed character profile for:

**Character Name:** {character_name}
**Role:** {role}
**Existing Details:** {existing_details}
**Book Context:** {book_context}

Please provide:

1. **Physical Description** (age, appearance, distinguishing features)
2. **Personality Traits** (3-5 key traits with examples)
3. **Background** (relevant history, upbringing)
4. **Motivations** (what drives them)
5. **Conflicts** (internal and external)
6. **Relationships** (how they relate to other characters)
7. **Character Arc** (how they change throughout the story)
8. **Voice** (speech patterns, vocabulary, mannerisms)

Make the character feel real and three-dimensional."#,
        character_name = character_name,
        role = role,
        existing_details = existing_details,
        book_context = book_context
    )
}

