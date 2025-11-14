# authorworks-audio-service

This repository is part of the AuthorWorks platform, which provides a comprehensive solution for authors to create, edit, and publish their content.

## Overview

The Audio Service provides audio-related functionality for the AuthorWorks platform.

The Audio Service implements the specifications documented in [Audio Service Specification](specs/2-services/2J-audio-service.md).

## Features

- **Text-to-Speech (TTS)**: Convert story text to natural-sounding audio using multiple voice engines
- **Audio Narration**: Professional narration generation with voice selection and emotion control
- **Multi-format Support**: Handle MP3, WAV, OGG, and AAC formats with automatic conversion
- **Audio Enhancement**: Noise reduction, normalization, and audio quality optimization
- **Background Music**: Ambient music generation and mixing for story scenes
- **Sound Effects Library**: Curated sound effects for story enhancement
- **Voice Cloning**: Create custom character voices from sample audio
- **Real-time Streaming**: Stream audio generation for immediate playback
- **Batch Processing**: Queue and process multiple audio generation tasks
- **Accessibility Features**: Generate audio descriptions for visual content

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo (Rust package manager)
- Dioxus CLI (`cargo install dioxus-cli`)
- Docker and Docker Compose

### Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/authorworks/authorworks-audio-service.git
   cd authorworks-audio-service
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
