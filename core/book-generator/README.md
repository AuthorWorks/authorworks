# Book Generator

Shared AI-powered book generation library for AuthorWorks platform.

## Overview

This library provides the core book generation functionality extracted from the legacy codebase. It implements a complete pipeline for AI-assisted creative writing:

1. **Braindump** - Initial story ideas and concepts
2. **Genre** - Genre classification and targeting
3. **Style** - Writing style definition
4. **Characters** - Character creation and development
5. **Synopsis** - Story synopsis generation
6. **Outline** - Chapter and scene outlining
7. **Content** - Actual content generation
8. **Rendering** - PDF/EPUB export

## Features

- Multiple LLM provider support (Anthropic Claude, OpenAI, Ollama)
- Token tracking and statistics
- Context-aware generation
- PDF and EPUB export
- Markdown rendering with mdBook

## Usage

```rust
use book_generator::{Book, Config};

// Create a new book
let config = Config::load()?;
let book = Book::new(config);

// Generate content
book.generate_from_braindump("Your story idea...")?;
```

## History

This code was originally duplicated across three service repositories:
- authorworks-audio-service/legacy-src
- authorworks-content-service/legacy-src
- authorworks-discovery-service/legacy-src

It has been consolidated here to eliminate ~26,000 LOC of duplication (94% of the codebase).

## License

See parent repository LICENSE file.
