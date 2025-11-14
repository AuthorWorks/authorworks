[![CI](https://github.com/AuthorWorks/authorworks-discovery-service/actions/workflows/ci.yml/badge.svg)](https://github.com/AuthorWorks/authorworks-discovery-service/actions/workflows/ci.yml) | [Umbrella](https://github.com/AuthorWorks/authorworks) | [Engine](https://github.com/AuthorWorks/authorworks-engine)

# authorworks-discovery-service

This repository is part of the AuthorWorks platform, which provides a comprehensive solution for authors to create, edit, and publish their content.

## Overview

The Discovery Service provides discovery-related functionality for the AuthorWorks platform.

The Discovery Service implements the specifications documented in [Discovery Service Specification](specs/2-services/2H-discovery-service.md).

## Features

- **Content Search**: Full-text search across stories, chapters, and metadata
- **AI Recommendations**: Personalized story recommendations based on reading history
- **Genre Classification**: Automatic genre and category detection
- **Trending Detection**: Identify trending topics and popular content
- **Similar Stories**: Find similar content using semantic analysis
- **Tag System**: Comprehensive tagging and categorization
- **Advanced Filters**: Filter by genre, length, rating, completion status
- **Author Discovery**: Find and follow favorite authors
- **Reading Lists**: Curated collections and user-generated lists
- **Search Analytics**: Track search patterns and popular queries
- **SEO Optimization**: Generate SEO-friendly metadata for content

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo (Rust package manager)
- Dioxus CLI (`cargo install dioxus-cli`)
- Docker and Docker Compose

### Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/authorworks/authorworks-discovery-service.git
   cd authorworks-discovery-service
   ```

2. Install dependencies:
   ```bash
   cargo build
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your local configuration
   ```

4. Start the development server:
   ```bash
   # For backend services
   cargo run
   
   # For UI components with Dioxus
   dx serve
   ```
