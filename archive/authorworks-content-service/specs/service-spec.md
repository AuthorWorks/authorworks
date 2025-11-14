# Technical Specification: 2C - Content Service

## Overview

The Content Service is the core component of the AuthorWorks platform, responsible for managing user-generated content, including stories, chapters, and associated metadata. It serves as the central repository for all content-related operations, from creation and editing to organization and version control.

## Objectives

- Provide a robust API for content management operations
- Enable efficient content storage and retrieval
- Support versioning and revision history
- Implement content organization (books, chapters, collections)
- Allow for content metadata management
- Enable collaborative editing functionality
- Support content publishing and access control

## Requirements

### 1. Core Content Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub author_id: Uuid,
    pub status: PublishStatus,
    pub metadata: BookMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadata {
    pub genre: Option<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub copyright: Option<String>,
    pub is_fiction: Option<bool>,
    pub word_count: u32,
    pub reading_time_minutes: u32,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublishStatus {
    Draft,
    InProgress,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Uuid,
    pub book_id: Uuid,
    pub title: String,
    pub position: u32,
    pub word_count: u32,
    pub status: PublishStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContent {
    pub chapter_id: Uuid,
    pub version: u32,
    pub content: String,
    pub content_format: ContentFormat,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentFormat {
    PlainText,
    Markdown,
    HTML,
    RichText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub is_public: bool,
    pub books: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub role: CollaboratorRole,
    pub invited_at: DateTime<Utc>,
    pub joined_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaboratorRole {
    Owner,
    Editor,
    Reviewer,
    Viewer,
}
```

### 2. Database Schema

```sql
CREATE TABLE books (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    cover_image_url VARCHAR(255),
    author_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    published_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE book_metadata (
    book_id UUID PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
    genre VARCHAR(100),
    language VARCHAR(50),
    copyright VARCHAR(255),
    is_fiction BOOLEAN,
    word_count INTEGER NOT NULL DEFAULT 0,
    reading_time_minutes INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE book_tags (
    book_id UUID REFERENCES books(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL,
    PRIMARY KEY (book_id, tag)
);

CREATE TABLE book_custom_fields (
    book_id UUID REFERENCES books(id) ON DELETE CASCADE,
    field_key VARCHAR(100) NOT NULL,
    field_value JSONB NOT NULL,
    PRIMARY KEY (book_id, field_key)
);

CREATE TABLE chapters (
    id UUID PRIMARY KEY,
    book_id UUID REFERENCES books(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    position INTEGER NOT NULL,
    word_count INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    published_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE chapter_contents (
    chapter_id UUID REFERENCES chapters(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_format VARCHAR(20) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by UUID NOT NULL,
    PRIMARY KEY (chapter_id, version)
);

CREATE TABLE collections (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    owner_id UUID NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE collection_books (
    collection_id UUID REFERENCES collections(id) ON DELETE CASCADE,
    book_id UUID REFERENCES books(id) ON DELETE CASCADE,
    added_at TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (collection_id, book_id)
);

CREATE TABLE collaborators (
    book_id UUID REFERENCES books(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role VARCHAR(20) NOT NULL,
    invited_at TIMESTAMP WITH TIME ZONE NOT NULL,
    joined_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (book_id, user_id)
);

CREATE INDEX idx_books_author ON books(author_id);
CREATE INDEX idx_books_status ON books(status);
CREATE INDEX idx_chapters_book ON chapters(book_id);
CREATE INDEX idx_chapters_position ON chapters(book_id, position);
CREATE INDEX idx_collections_owner ON collections(owner_id);
```

### 3. API Endpoints

```
# Books
POST   /v1/books                       - Create a new book
GET    /v1/books                       - List books (filtered by user)
GET    /v1/books/{id}                  - Get book details
PUT    /v1/books/{id}                  - Update book details
DELETE /v1/books/{id}                  - Delete a book
PUT    /v1/books/{id}/cover            - Upload book cover image
PUT    /v1/books/{id}/publish          - Publish a book
PUT    /v1/books/{id}/unpublish        - Unpublish a book

# Chapters
POST   /v1/books/{book_id}/chapters    - Create a new chapter
GET    /v1/books/{book_id}/chapters    - List chapters in a book
GET    /v1/chapters/{id}               - Get chapter details
PUT    /v1/chapters/{id}               - Update chapter details
DELETE /v1/chapters/{id}               - Delete a chapter
PUT    /v1/chapters/{id}/position      - Update chapter position
PUT    /v1/chapters/{id}/publish       - Publish a chapter
PUT    /v1/chapters/{id}/unpublish     - Unpublish a chapter

# Chapter Content
GET    /v1/chapters/{id}/content       - Get latest chapter content
GET    /v1/chapters/{id}/content/{version} - Get specific version of content
PUT    /v1/chapters/{id}/content       - Update chapter content
GET    /v1/chapters/{id}/versions      - List content versions

# Collections
POST   /v1/collections                 - Create a new collection
GET    /v1/collections                 - List user's collections
GET    /v1/collections/public          - List public collections
GET    /v1/collections/{id}            - Get collection details
PUT    /v1/collections/{id}            - Update collection details
DELETE /v1/collections/{id}            - Delete a collection
POST   /v1/collections/{id}/books/{book_id} - Add book to collection
DELETE /v1/collections/{id}/books/{book_id} - Remove book from collection

# Collaboration
POST   /v1/books/{id}/collaborators    - Invite collaborator
GET    /v1/books/{id}/collaborators    - List collaborators
PUT    /v1/books/{id}/collaborators/{user_id} - Update collaborator role
DELETE /v1/books/{id}/collaborators/{user_id} - Remove collaborator
GET    /v1/collaborations              - List books user collaborates on
POST   /v1/collaborations/invites/{id}/accept - Accept collaboration invite
POST   /v1/collaborations/invites/{id}/reject - Reject collaboration invite
```

### 4. Content Management Services

#### BookService

```rust
pub struct BookService {
    book_repository: Arc<dyn BookRepository>,
    chapter_repository: Arc<dyn ChapterRepository>,
    metadata_repository: Arc<dyn BookMetadataRepository>,
    storage_service: Arc<dyn StorageService>,
    search_service: Arc<dyn SearchService>,
}

impl BookService {
    pub async fn create_book(&self, request: CreateBookRequest, user_id: Uuid) -> Result<Book, Error> {
        // Validate request
        if request.title.is_empty() {
            return Err(Error::ValidationFailed("Title cannot be empty".to_string()));
        }
        
        // Create book
        let book_id = Uuid::new_v4();
        let now = Utc::now();
        
        let book = Book {
            id: book_id,
            title: request.title,
            description: request.description,
            cover_image_url: None,
            author_id: user_id,
            status: PublishStatus::Draft,
            metadata: BookMetadata {
                genre: request.genre,
                tags: request.tags.unwrap_or_default(),
                language: request.language,
                copyright: None,
                is_fiction: request.is_fiction,
                word_count: 0,
                reading_time_minutes: 0,
                custom_fields: HashMap::new(),
            },
            created_at: now,
            updated_at: now,
            published_at: None,
        };
        
        // Save to repository
        let created_book = self.book_repository.create(book).await?;
        
        // Index in search
        self.search_service.index_book(&created_book).await?;
        
        Ok(created_book)
    }
    
    pub async fn get_book(&self, book_id: Uuid, user_id: Uuid) -> Result<Book, Error> {
        let book = self.book_repository.find_by_id(book_id).await?;
        
        // Check permissions
        if !self.can_view_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        Ok(book)
    }
    
    pub async fn update_book(&self, book_id: Uuid, request: UpdateBookRequest, user_id: Uuid) -> Result<Book, Error> {
        let mut book = self.book_repository.find_by_id(book_id).await?;
        
        // Check permissions
        if !self.can_edit_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Update fields
        if let Some(title) = request.title {
            book.title = title;
        }
        
        if let Some(description) = request.description {
            book.description = Some(description);
        }
        
        if let Some(genre) = request.genre {
            book.metadata.genre = Some(genre);
        }
        
        if let Some(tags) = request.tags {
            book.metadata.tags = tags;
        }
        
        if let Some(language) = request.language {
            book.metadata.language = Some(language);
        }
        
        if let Some(is_fiction) = request.is_fiction {
            book.metadata.is_fiction = Some(is_fiction);
        }
        
        book.updated_at = Utc::now();
        
        // Save to repository
        let updated_book = self.book_repository.update(book).await?;
        
        // Update search index
        self.search_service.update_book_index(&updated_book).await?;
        
        Ok(updated_book)
    }
    
    pub async fn delete_book(&self, book_id: Uuid, user_id: Uuid) -> Result<(), Error> {
        let book = self.book_repository.find_by_id(book_id).await?;
        
        // Check permissions (only author or admin can delete)
        if book.author_id != user_id && !self.is_admin(user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Delete from repository
        self.book_repository.delete(book_id).await?;
        
        // Remove from search index
        self.search_service.remove_book_from_index(book_id).await?;
        
        Ok(())
    }
    
    pub async fn upload_cover_image(
        &self, 
        book_id: Uuid, 
        image: UploadedFile, 
        user_id: Uuid,
    ) -> Result<String, Error> {
        let book = self.book_repository.find_by_id(book_id).await?;
        
        // Check permissions
        if !self.can_edit_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Validate image
        if !["image/jpeg", "image/png", "image/webp"].contains(&image.content_type.as_str()) {
            return Err(Error::UnsupportedMediaType);
        }
        
        if image.content.len() > 5 * 1024 * 1024 {
            return Err(Error::FileTooLarge);
        }
        
        // Upload to storage
        let path = format!("book-covers/{}/{}", book_id, Uuid::new_v4());
        let url = self.storage_service.upload_file(&path, &image.content).await?;
        
        // Update book record
        let mut updated_book = book.clone();
        updated_book.cover_image_url = Some(url.clone());
        updated_book.updated_at = Utc::now();
        
        self.book_repository.update(updated_book).await?;
        
        Ok(url)
    }
    
    // Permission checking helpers
    async fn can_view_book(&self, book: &Book, user_id: Uuid) -> Result<bool, Error> {
        // Public published books can be viewed by anyone
        if book.status == PublishStatus::Published {
            return Ok(true);
        }
        
        // Author can always view their book
        if book.author_id == user_id {
            return Ok(true);
        }
        
        // Check if user is a collaborator
        let collaborators = self.book_repository.get_collaborators(book.id).await?;
        if collaborators.iter().any(|c| c.user_id == user_id) {
            return Ok(true);
        }
        
        // Otherwise, access denied
        Ok(false)
    }
    
    async fn can_edit_book(&self, book: &Book, user_id: Uuid) -> Result<bool, Error> {
        // Author can always edit
        if book.author_id == user_id {
            return Ok(true);
        }
        
        // Check if user is a collaborator with edit permissions
        let collaborators = self.book_repository.get_collaborators(book.id).await?;
        if collaborators.iter().any(|c| 
            c.user_id == user_id && 
            (c.role == CollaboratorRole::Editor || c.role == CollaboratorRole::Owner)
        ) {
            return Ok(true);
        }
        
        // Otherwise, cannot edit
        Ok(false)
    }
    
    async fn is_admin(&self, user_id: Uuid) -> Result<bool, Error> {
        // In a real implementation, this would check user roles
        // For now, we'll assume no one is an admin for safety
        Ok(false)
    }
}
```

#### ChapterService

```rust
pub struct ChapterService {
    chapter_repository: Arc<dyn ChapterRepository>,
    content_repository: Arc<dyn ChapterContentRepository>,
    book_repository: Arc<dyn BookRepository>,
    book_service: Arc<dyn BookService>,
}

impl ChapterService {
    pub async fn create_chapter(
        &self, 
        request: CreateChapterRequest, 
        user_id: Uuid,
    ) -> Result<Chapter, Error> {
        // Verify book exists and user has permissions
        let book = self.book_repository.find_by_id(request.book_id).await?;
        if !self.book_service.can_edit_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Get position for new chapter
        let position = if let Some(position) = request.position {
            // If position specified, use it
            position
        } else {
            // Otherwise, append to end
            let existing_chapters = self.chapter_repository.find_by_book_id(book.id).await?;
            let max_position = existing_chapters.iter()
                .map(|ch| ch.position)
                .max()
                .unwrap_or(0);
            max_position + 1
        };
        
        // Create chapter
        let chapter_id = Uuid::new_v4();
        let now = Utc::now();
        
        let chapter = Chapter {
            id: chapter_id,
            book_id: book.id,
            title: request.title,
            position,
            word_count: 0,
            status: PublishStatus::Draft,
            created_at: now,
            updated_at: now,
            published_at: None,
        };
        
        // Create initial empty content
        let content = ChapterContent {
            chapter_id,
            version: 1,
            content: request.initial_content.unwrap_or_default(),
            content_format: request.content_format.unwrap_or(ContentFormat::RichText),
            created_at: now,
            created_by: user_id,
        };
        
        // Save to repositories
        let created_chapter = self.chapter_repository.create(chapter).await?;
        self.content_repository.create(content).await?;
        
        // If position was inserted in the middle, update other chapters
        if position <= existing_chapters.len() as u32 {
            self.update_chapter_positions(book.id, position, 1).await?;
        }
        
        Ok(created_chapter)
    }
    
    pub async fn get_chapter(&self, chapter_id: Uuid, user_id: Uuid) -> Result<Chapter, Error> {
        let chapter = self.chapter_repository.find_by_id(chapter_id).await?;
        
        // Check book access permissions
        let book = self.book_repository.find_by_id(chapter.book_id).await?;
        if !self.book_service.can_view_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        Ok(chapter)
    }
    
    pub async fn get_chapter_content(
        &self, 
        chapter_id: Uuid, 
        version: Option<u32>, 
        user_id: Uuid,
    ) -> Result<ChapterContent, Error> {
        // Check permissions first
        let chapter = self.chapter_repository.find_by_id(chapter_id).await?;
        let book = self.book_repository.find_by_id(chapter.book_id).await?;
        
        if !self.book_service.can_view_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Retrieve content
        let content = if let Some(v) = version {
            self.content_repository.find_by_chapter_id_and_version(chapter_id, v).await?
        } else {
            self.content_repository.find_latest_by_chapter_id(chapter_id).await?
        };
        
        Ok(content)
    }
    
    pub async fn update_chapter_content(
        &self, 
        chapter_id: Uuid, 
        request: UpdateContentRequest, 
        user_id: Uuid,
    ) -> Result<ChapterContent, Error> {
        // Check permissions
        let chapter = self.chapter_repository.find_by_id(chapter_id).await?;
        let book = self.book_repository.find_by_id(chapter.book_id).await?;
        
        if !self.book_service.can_edit_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Get current version
        let current_content = self.content_repository.find_latest_by_chapter_id(chapter_id).await?;
        
        // Create new version
        let word_count = count_words(&request.content);
        let now = Utc::now();
        
        let new_content = ChapterContent {
            chapter_id,
            version: current_content.version + 1,
            content: request.content,
            content_format: request.content_format.unwrap_or(current_content.content_format),
            created_at: now,
            created_by: user_id,
        };
        
        // Update chapter word count
        let mut updated_chapter = chapter.clone();
        updated_chapter.word_count = word_count;
        updated_chapter.updated_at = now;
        
        // Save to repositories
        self.content_repository.create(new_content.clone()).await?;
        self.chapter_repository.update(updated_chapter).await?;
        
        // Update book word count
        self.update_book_word_count(book.id).await?;
        
        Ok(new_content)
    }
    
    pub async fn update_chapter_position(
        &self, 
        chapter_id: Uuid, 
        new_position: u32, 
        user_id: Uuid,
    ) -> Result<Chapter, Error> {
        // Check permissions
        let chapter = self.chapter_repository.find_by_id(chapter_id).await?;
        let book = self.book_repository.find_by_id(chapter.book_id).await?;
        
        if !self.book_service.can_edit_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        if chapter.position == new_position {
            return Ok(chapter); // No change needed
        }
        
        // Update positions of other chapters
        if new_position < chapter.position {
            // Moving up - increment chapters in between
            self.shift_chapter_positions(
                book.id, 
                new_position, 
                chapter.position - 1, 
                1
            ).await?;
        } else {
            // Moving down - decrement chapters in between
            self.shift_chapter_positions(
                book.id, 
                chapter.position + 1, 
                new_position, 
                -1
            ).await?;
        }
        
        // Update this chapter's position
        let mut updated_chapter = chapter.clone();
        updated_chapter.position = new_position;
        updated_chapter.updated_at = Utc::now();
        
        let result = self.chapter_repository.update(updated_chapter).await?;
        Ok(result)
    }
    
    // Helper to update positions when inserting or moving chapters
    async fn update_chapter_positions(
        &self, 
        book_id: Uuid, 
        start_position: u32, 
        offset: i32,
    ) -> Result<(), Error> {
        let chapters = self.chapter_repository.find_by_book_id(book_id).await?;
        
        for chapter in chapters.iter().filter(|c| c.position >= start_position) {
            let new_position = (chapter.position as i32 + offset) as u32;
            let mut updated = chapter.clone();
            updated.position = new_position;
            self.chapter_repository.update(updated).await?;
        }
        
        Ok(())
    }
    
    // Helper to shift positions in a range
    async fn shift_chapter_positions(
        &self,
        book_id: Uuid,
        start_position: u32,
        end_position: u32,
        offset: i32,
    ) -> Result<(), Error> {
        let chapters = self.chapter_repository.find_by_book_id(book_id).await?;
        
        for chapter in chapters.iter().filter(|c| 
            c.position >= start_position && c.position <= end_position
        ) {
            let new_position = (chapter.position as i32 + offset) as u32;
            let mut updated = chapter.clone();
            updated.position = new_position;
            self.chapter_repository.update(updated).await?;
        }
        
        Ok(())
    }
    
    // Update book's total word count
    async fn update_book_word_count(&self, book_id: Uuid) -> Result<(), Error> {
        let chapters = self.chapter_repository.find_by_book_id(book_id).await?;
        let total_words: u32 = chapters.iter().map(|c| c.word_count).sum();
        
        let mut book = self.book_repository.find_by_id(book_id).await?;
        book.metadata.word_count = total_words;
        
        // Estimate reading time (average reading speed of 200-250 words per minute)
        book.metadata.reading_time_minutes = (total_words + 225) / 250; // Round up
        book.updated_at = Utc::now();
        
        self.book_repository.update(book).await?;
        
        Ok(())
    }
}

// Helper function to count words in text
fn count_words(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}
```

### 5. Search and Discovery

```rust
pub struct ContentSearchService {
    search_client: Arc<SearchClient>,
    book_repository: Arc<dyn BookRepository>,
}

impl SearchService for ContentSearchService {
    pub async fn index_book(&self, book: &Book) -> Result<(), Error> {
        let document = BookSearchDocument {
            id: book.id.to_string(),
            title: book.title.clone(),
            description: book.description.clone().unwrap_or_default(),
            author_id: book.author_id.to_string(),
            status: format!("{:?}", book.status),
            genre: book.metadata.genre.clone().unwrap_or_default(),
            tags: book.metadata.tags.clone(),
            language: book.metadata.language.clone().unwrap_or_default(),
            is_fiction: book.metadata.is_fiction.unwrap_or(true),
            word_count: book.metadata.word_count,
            created_at: book.created_at.to_rfc3339(),
            published_at: book.published_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        };
        
        self.search_client.index_document("books", &book.id.to_string(), &document).await?;
        Ok(())
    }
    
    pub async fn update_book_index(&self, book: &Book) -> Result<(), Error> {
        // Same implementation as index_book
        self.index_book(book).await
    }
    
    pub async fn remove_book_from_index(&self, book_id: Uuid) -> Result<(), Error> {
        self.search_client.delete_document("books", &book_id.to_string()).await?;
        Ok(())
    }
    
    pub async fn search_books(
        &self, 
        query: &str, 
        filters: &BookSearchFilters, 
        pagination: &Pagination,
    ) -> Result<SearchResults<Book>, Error> {
        // Build search query
        let mut search_query = self.search_client.search("books")
            .query(query)
            .offset(pagination.offset)
            .limit(pagination.limit);
        
        // Apply filters
        if let Some(status) = &filters.status {
            search_query = search_query.filter("status", format!("{:?}", status));
        }
        
        if let Some(genre) = &filters.genre {
            search_query = search_query.filter("genre", genre);
        }
        
        if let Some(tags) = &filters.tags {
            for tag in tags {
                search_query = search_query.filter("tags", tag);
            }
        }
        
        if let Some(author_id) = filters.author_id {
            search_query = search_query.filter("author_id", author_id.to_string());
        }
        
        // Execute search
        let results = search_query.execute().await?;
        
        // Convert results back to Book objects
        let book_ids: Vec<Uuid> = results.hits.iter()
            .filter_map(|hit| Uuid::parse_str(&hit.id).ok())
            .collect();
        
        let books = self.book_repository.find_by_ids(&book_ids).await?;
        
        // Return results
        Ok(SearchResults {
            total: results.total,
            items: books,
        })
    }
}
```

### 6. Collaborative Editing

```rust
pub struct CollaborationService {
    book_repository: Arc<dyn BookRepository>,
    collaborator_repository: Arc<dyn CollaboratorRepository>,
    notification_service: Arc<dyn NotificationService>,
}

impl CollaborationService {
    pub async fn invite_collaborator(
        &self, 
        book_id: Uuid, 
        email: &str, 
        role: CollaboratorRole, 
        inviter_id: Uuid,
    ) -> Result<Collaborator, Error> {
        // Verify book exists and inviter has permission
        let book = self.book_repository.find_by_id(book_id).await?;
        
        if book.author_id != inviter_id {
            // Check if inviter is an owner-level collaborator
            let inviter_collab = self.collaborator_repository
                .find_by_book_and_user(book_id, inviter_id)
                .await?;
                
            if inviter_collab.map(|c| c.role) != Some(CollaboratorRole::Owner) {
                return Err(Error::PermissionDenied);
            }
        }
        
        // Look up user by email
        let user = self.user_service.find_by_email(email).await?;
        
        // Check if already a collaborator
        if let Some(_) = self.collaborator_repository
            .find_by_book_and_user(book_id, user.id)
            .await? {
            return Err(Error::AlreadyExists("User is already a collaborator".to_string()));
        }
        
        // Create collaboration invite
        let collaborator = Collaborator {
            book_id,
            user_id: user.id,
            role,
            invited_at: Utc::now(),
            joined_at: None,
        };
        
        let created = self.collaborator_repository.create(collaborator).await?;
        
        // Send notification to invited user
        self.notification_service.send_collaboration_invite(
            user.id,
            inviter_id,
            book_id,
            book.title.clone(),
            role,
        ).await?;
        
        Ok(created)
    }
    
    pub async fn accept_invitation(
        &self, 
        book_id: Uuid, 
        user_id: Uuid,
    ) -> Result<Collaborator, Error> {
        // Find invitation
        let collaboration = self.collaborator_repository
            .find_by_book_and_user(book_id, user_id)
            .await?
            .ok_or(Error::NotFound("Collaboration invitation not found".to_string()))?;
        
        if collaboration.joined_at.is_some() {
            return Err(Error::InvalidState("Invitation already accepted".to_string()));
        }
        
        // Update collaboration record
        let updated = self.collaborator_repository
            .update_joined_at(book_id, user_id, Utc::now())
            .await?;
        
        // Notify book owner
        let book = self.book_repository.find_by_id(book_id).await?;
        self.notification_service.send_collaboration_accepted(
            book.author_id,
            user_id,
            book_id,
            book.title,
        ).await?;
        
        Ok(updated)
    }
    
    pub async fn reject_invitation(
        &self, 
        book_id: Uuid, 
        user_id: Uuid,
    ) -> Result<(), Error> {
        // Find invitation
        let collaboration = self.collaborator_repository
            .find_by_book_and_user(book_id, user_id)
            .await?
            .ok_or(Error::NotFound("Collaboration invitation not found".to_string()))?;
        
        if collaboration.joined_at.is_some() {
            return Err(Error::InvalidState("Invitation already accepted".to_string()));
        }
        
        // Delete collaboration record
        self.collaborator_repository.delete(book_id, user_id).await?;
        
        // Notify book owner
        let book = self.book_repository.find_by_id(book_id).await?;
        self.notification_service.send_collaboration_rejected(
            book.author_id,
            user_id,
            book_id,
            book.title,
        ).await?;
        
        Ok(())
    }
    
    pub async fn get_book_collaborators(
        &self, 
        book_id: Uuid, 
        user_id: Uuid,
    ) -> Result<Vec<CollaboratorWithUser>, Error> {
        // Verify book exists and user has permission to view
        let book = self.book_repository.find_by_id(book_id).await?;
        let book_service = self.book_service.clone();
        
        if !book_service.can_view_book(&book, user_id).await? {
            return Err(Error::PermissionDenied);
        }
        
        // Get collaborators
        let collaborators = self.collaborator_repository
            .find_by_book_id_with_users(book_id)
            .await?;
        
        Ok(collaborators)
    }
    
    pub async fn get_user_collaborations(
        &self, 
        user_id: Uuid,
    ) -> Result<Vec<BookWithCollaborator>, Error> {
        // Get all collaborations for this user
        let collaborations = self.collaborator_repository
            .find_by_user_id_with_books(user_id)
            .await?;
        
        Ok(collaborations)
    }
}
```

## Implementation Steps

1. Create Content Service project structure
2. Implement database models and repositories
3. Set up storage integration for book covers
4. Implement BookService for core book operations
5. Create ChapterService for chapter and content management
6. Add search indexing and discovery features
7. Implement collaborative editing functionality
8. Create collection management features
9. Add versioning system for content
10. Implement comprehensive access control
11. Set up content analytics tracking
12. Create robust testing suite

## Technical Decisions

### Why Store Content History?

Chapter content versioning was chosen because:
- Enables tracking of revision history for audit and recovery
- Allows users to revert to previous versions if needed
- Provides foundation for collaborative editing with change tracking
- Supports analytics about content development over time
- Enables "time travel" features for reviewing writing progress

### Why PostgreSQL with JSON for Content?

PostgreSQL was selected for content storage with these considerations:
- JSONB type provides efficient storage for rich text content
- Transactions ensure consistency across related updates
- Full-text search capabilities for basic content searching
- Ability to store metadata alongside content
- Simple versioning through append-only records

## Success Criteria

The Content Service will be considered successfully implemented when:

1. Authors can create, edit and organize their books and chapters
2. Content versioning works correctly with full history support
3. Collaborative editing functions properly with appropriate permissions
4. Content can be published, unpublished and archived as needed
5. Book collections can be created and managed
6. Search functionality returns relevant results efficiently
7. Content analytics provides meaningful insights
8. Service handles high load with minimal latency (<200ms for content retrieval)
9. All operations properly respect access control rules 