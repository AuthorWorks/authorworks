[![CI](https://github.com/AuthorWorks/authorworks-content-service/actions/workflows/ci.yml/badge.svg)](https://github.com/AuthorWorks/authorworks-content-service/actions/workflows/ci.yml) | [Umbrella](https://github.com/AuthorWorks/authorworks) | [Engine](https://github.com/AuthorWorks/authorworks-engine)

# authorworks-content-service

This repository is part of the AuthorWorks platform, which provides a comprehensive solution for authors to create, edit, and publish their content.

## Overview

The Content Service manages all content-related operations in the AuthorWorks platform, including creation, storage, versioning, and metadata management.

The Content Service implements the specifications documented in [Content Service Specification](specs/2-services/2C-content-service.md).

Additionally, it implements the following business logic:

- [Repository Distribution](specs/3-business-logic/3A-repository-distribution.md)
- [Publishing Workflow](specs/3-business-logic/3B-publishing-workflow.md)

## Features

- Content creation and editing
- Version control
- Metadata management
- Content transformation
- Publishing workflows

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo (Rust package manager)
- Dioxus CLI (`cargo install dioxus-cli`)
- Docker and Docker Compose

### Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/authorworks/authorworks-content-service.git
   cd authorworks-content-service
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
