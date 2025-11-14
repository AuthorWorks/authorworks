# Discovery Service Specification

## 1. Overview

The Discovery Service provides content discovery, search, and recommendation capabilities for the AuthorWorks platform. It enables users to discover relevant content, find creators to follow, and receive personalized recommendations based on their preferences and behavior.

## 2. Objectives

- Provide fast, accurate search across all platform content
- Generate personalized content recommendations for users
- Support content browsing with filtering and sorting options
- Track trending and popular content
- Facilitate content categorization and tagging
- Optimize content discoverability and user engagement

## 3. Core Components

### 3.1 Search System

The search system will provide:

- Full-text search across content with relevance ranking
- Faceted search with multiple filters
- Typo tolerance and auto-correction
- Search analytics and query optimization
- Real-time indexing of new content

### 3.2 Recommendation Engine

The recommendation engine will:

- Process user interaction events to learn preferences
- Generate personalized content recommendations
- Support collaborative filtering approaches
- Implement content-based recommendations
- Balance discovery of new content with known user preferences
- Include diversity in recommendations to avoid filter bubbles

### 3.3 Content Categorization

The categorization system will:

- Maintain a hierarchical genre taxonomy
- Apply automatic genre classification
- Support content tagging and metadata
- Implement content similarity analysis
- Provide topic extraction and clustering

### 3.4 Trending Analysis

The trending system will:

- Track content popularity metrics in real-time
- Identify rapidly growing content
- Balance recency with absolute popularity
- Support trending categories and tags
- Provide time-window analysis (daily, weekly, monthly trends)

## 4. Database Schema

### Search Indexes

```sql
-- This represents logical structure - actual implementation 
-- will use dedicated search engine technology (e.g., Elasticsearch)
CREATE TABLE search_indexes (
    id UUID PRIMARY KEY,
    content_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    body TEXT,
    author_id UUID NOT NULL,
    tags TEXT[],
    genres TEXT[],
    created_at TIMESTAMP WITH TIME ZONE,
    published_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE,
    popularity_score FLOAT,
    quality_score FLOAT,
    vector_embedding VECTOR(384), -- For semantic search
    searchable_text TEXT NOT NULL -- Concatenated searchable content
);
```

### User Interactions Table

```sql
CREATE TABLE user_interactions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES user_service.users(id),
    content_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    interaction_type TEXT NOT NULL CHECK (interaction_type IN ('view', 'read', 'like', 'share', 'follow', 'subscribe', 'comment', 'bookmark')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB,
    session_id UUID,
    interaction_strength FLOAT -- Normalized strength of interaction
);
```

### Content Recommendations Table

```sql
CREATE TABLE content_recommendations (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES user_service.users(id),
    content_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    recommendation_type TEXT NOT NULL CHECK (recommendation_type IN ('personalized', 'trending', 'similar', 'collaborative', 'editorial')),
    score FLOAT NOT NULL,
    reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    is_displayed BOOLEAN DEFAULT FALSE,
    is_clicked BOOLEAN DEFAULT FALSE
);
```

### Trending Content Table

```sql
CREATE TABLE trending_content (
    id UUID PRIMARY KEY,
    content_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    trending_score FLOAT NOT NULL,
    period TEXT NOT NULL CHECK (period IN ('hourly', 'daily', 'weekly', 'monthly')),
    rank INTEGER NOT NULL,
    previous_rank INTEGER,
    genre TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE
);
```

### Content Categories Table

```sql
CREATE TABLE content_categories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    parent_id UUID REFERENCES content_categories(id),
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

### Content Tags Table

```sql
CREATE TABLE content_tags (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    normalized_name TEXT NOT NULL UNIQUE,
    usage_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

### Content-Tag Relations Table

```sql
CREATE TABLE content_tag_relations (
    content_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    tag_id UUID NOT NULL REFERENCES content_tags(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (content_id, tag_id)
);
```

## 5. API Endpoints

### 5.1 Search

#### Search Content

```
GET /api/v1/discovery/search
```

Parameters:
- `query`: Search text (required)
- `content_types`: Array of content types to include (optional, default: all)
- `genres`: Array of genres to filter by (optional)
- `tags`: Array of tags to filter by (optional)
- `author_id`: Filter by author ID (optional)
- `sort`: Sort field (optional, default: relevance)
- `order`: Sort order (optional, default: desc)
- `page`: Page number (optional, default: 1)
- `per_page`: Items per page (optional, default: 20)

Response:
```json
{
  "results": [
    {
      "id": "content-uuid",
      "title": "Content Title",
      "description": "Content description with <em>highlighted</em> search terms",
      "content_type": "book",
      "author": {
        "id": "author-uuid",
        "name": "Author Name"
      },
      "cover_image": "url-to-cover-image",
      "created_at": "ISO8601",
      "relevance_score": 0.95,
      "genres": ["fantasy", "adventure"],
      "tags": ["magic", "quest"],
      "highlights": [
        "Text excerpt with <em>highlighted</em> search terms..."
      ]
    }
  ],
  "facets": {
    "content_types": [
      {"key": "book", "count": 120},
      {"key": "graphic_novel", "count": 45}
    ],
    "genres": [
      {"key": "fantasy", "count": 78},
      {"key": "science_fiction", "count": 42}
    ],
    "tags": [
      {"key": "magic", "count": 35},
      {"key": "space", "count": 27}
    ]
  },
  "total": 165,
  "page": 1,
  "per_page": 20,
  "total_pages": 9,
  "query_time_ms": 87
}
```

#### Autocomplete

```
GET /api/v1/discovery/autocomplete
```

Parameters:
- `query`: Partial search text (required)
- `type`: Type of autocomplete (optional, default: content, options: content, tag, author)
- `limit`: Maximum number of suggestions (optional, default: 10)

Response:
```json
{
  "suggestions": [
    {
      "text": "Harry Potter",
      "highlighted": "<em>Harry</em> Potter",
      "type": "content",
      "id": "content-uuid"
    },
    {
      "text": "Harry Styles Fan Fiction",
      "highlighted": "<em>Harry</em> Styles Fan Fiction",
      "type": "content",
      "id": "content-uuid-2"
    }
  ],
  "query_time_ms": 12
}
```

### 5.2 Recommendations

#### Get Personalized Recommendations

```
GET /api/v1/discovery/recommendations/personalized
```

Parameters:
- `user_id`: User ID (required)
- `content_types`: Array of content types to include (optional, default: all)
- `limit`: Maximum number of recommendations (optional, default: 20)
- `excluded_ids`: Array of content IDs to exclude (optional)

Response:
```json
{
  "recommendations": [
    {
      "id": "content-uuid",
      "title": "Content Title",
      "description": "Content description",
      "content_type": "book",
      "author": {
        "id": "author-uuid",
        "name": "Author Name"
      },
      "cover_image": "url-to-cover-image",
      "created_at": "ISO8601",
      "recommendation_score": 0.92,
      "reason": "Because you read Similar Title",
      "genres": ["fantasy", "adventure"]
    }
  ],
  "total": 20
}
```

#### Get Similar Content

```
GET /api/v1/discovery/recommendations/similar/{content_id}
```

Parameters:
- `limit`: Maximum number of similar items (optional, default: 10)
- `min_similarity`: Minimum similarity score (optional, default: 0.7)

Response:
```json
{
  "similar_content": [
    {
      "id": "content-uuid",
      "title": "Similar Content Title",
      "description": "Content description",
      "content_type": "book",
      "author": {
        "id": "author-uuid",
        "name": "Author Name"
      },
      "cover_image": "url-to-cover-image",
      "created_at": "ISO8601",
      "similarity_score": 0.85,
      "common_features": ["same genre", "similar themes"]
    }
  ],
  "total": 10
}
```

### 5.3 Trending Content

#### Get Trending Content

```
GET /api/v1/discovery/trending
```

Parameters:
- `period`: Time period (optional, default: daily, options: hourly, daily, weekly, monthly)
- `genre`: Filter by genre (optional)
- `content_types`: Array of content types to include (optional, default: all)
- `limit`: Maximum number of trending items (optional, default: 20)

Response:
```json
{
  "trending": [
    {
      "id": "content-uuid",
      "title": "Trending Content Title",
      "description": "Content description",
      "content_type": "book",
      "author": {
        "id": "author-uuid",
        "name": "Author Name"
      },
      "cover_image": "url-to-cover-image",
      "created_at": "ISO8601",
      "trending_score": 95.7,
      "rank": 1,
      "previous_rank": 3,
      "movement": "up",
      "genres": ["fantasy", "adventure"]
    }
  ],
  "period": "daily",
  "generated_at": "ISO8601",
  "total": 20
}
```

### 5.4 Categories and Tags

#### List Categories

```
GET /api/v1/discovery/categories
```

Parameters:
- `parent_id`: Filter by parent category ID (optional)
- `include_content_count`: Include content count in each category (optional, default: false)

Response:
```json
{
  "categories": [
    {
      "id": "category-uuid",
      "name": "Fantasy",
      "slug": "fantasy",
      "description": "Fantasy genre description",
      "parent_id": null,
      "content_count": 1245,
      "subcategories": [
        {
          "id": "subcategory-uuid",
          "name": "Epic Fantasy",
          "slug": "epic-fantasy",
          "description": "Epic fantasy subgenre description",
          "content_count": 421
        }
      ]
    }
  ],
  "total": 15
}
```

#### Get Popular Tags

```
GET /api/v1/discovery/tags/popular
```

Parameters:
- `limit`: Maximum number of tags (optional, default: 50)
- `content_type`: Filter by content type (optional)

Response:
```json
{
  "tags": [
    {
      "id": "tag-uuid",
      "name": "magic",
      "usage_count": 1245
    },
    {
      "id": "tag-uuid-2",
      "name": "dragons",
      "usage_count": 987
    }
  ],
  "total": 50
}
```

### 5.5 User Interactions

#### Record User Interaction

```
POST /api/v1/discovery/interactions
```

Request:
```json
{
  "user_id": "user-uuid",
  "content_id": "content-uuid",
  "content_type": "book",
  "interaction_type": "view",
  "metadata": {
    "view_time_seconds": 120,
    "completion_percentage": 0.15,
    "source": "recommendation"
  },
  "session_id": "session-uuid"
}
```

Response:
```json
{
  "id": "interaction-uuid",
  "created_at": "ISO8601",
  "success": true
}
```

#### Get User Interaction History

```
GET /api/v1/discovery/users/{user_id}/interactions
```

Parameters:
- `interaction_types`: Array of interaction types to include (optional, default: all)
- `content_types`: Array of content types to include (optional, default: all)
- `limit`: Maximum number of interactions (optional, default: 50)
- `offset`: Pagination offset (optional, default: 0)

Response:
```json
{
  "interactions": [
    {
      "id": "interaction-uuid",
      "content": {
        "id": "content-uuid",
        "title": "Content Title",
        "content_type": "book"
      },
      "interaction_type": "view",
      "created_at": "ISO8601",
      "metadata": {
        "view_time_seconds": 120,
        "completion_percentage": 0.15
      }
    }
  ],
  "total": 237,
  "limit": 50,
  "offset": 0
}
```

## 6. Integration with Other Services

### 6.1 Content Service Integration

- Content indexing when new content is created or updated
- Retrieval of content details for search results and recommendations
- Content similarity analysis for "more like this" recommendations
- Content quality scoring for ranking algorithms

### 6.2 User Service Integration

- User preference data for personalized recommendations
- User follow/subscription data for social recommendations
- User demographic information for cohort analysis
- User authentication and permissions

### 6.3 Subscription Service Integration

- Creator subscription data for premium content recommendations
- Subscription tier information for access control
- Revenue data for trending and popular content analysis
- Subscriber analytics for creator recommendations

### 6.4 UI Shell Integration

- Search interface components
- Recommendation panels and carousels
- Trending content displays
- Category and tag browsers

## 7. Search Engine Technology

### 7.1 Implementation Options

- **Elasticsearch**: Powerful search engine with advanced features
- **Meilisearch**: Fast and user-friendly search with typo tolerance
- **PostgreSQL + pgvector**: Integrated solution with vector search capabilities
- **Tantivy + HNSW**: Rust-native search with approximate nearest neighbors

### 7.2 Vector Embeddings

- Model selection for semantic search (e.g., BERT, Sentence Transformers)
- Embedding generation pipeline
- Efficient vector storage and retrieval
- Hybrid search combining vector similarity with keyword matching

### 7.3 Indexing Pipeline

- Content change detection
- Text extraction and normalization
- Classification and tagging
- Vector embedding generation
- Index update strategy

## 8. Recommendation System

### 8.1 Data Collection

- Event tracking for user interactions
- Session management
- Privacy-preserving data collection
- Feature extraction from content and user behavior

### 8.2 Algorithm Selection

- Collaborative filtering approaches
- Content-based recommendation
- Hybrid recommendation systems
- Contextual bandits for exploration vs. exploitation
- Deep learning approaches for complex patterns

### 8.3 Training Pipeline

- Feature engineering
- Model training infrastructure
- Evaluation metrics
- A/B testing framework
- Model deployment and serving

### 8.4 Recommendation Diversity

- Re-ranking techniques for diversity
- Novelty metrics
- Serendipity optimization
- Filter bubble prevention
- Category and author balancing

## 9. Implementation Steps

1. Design and implement search indexes
2. Develop content indexing pipeline
3. Implement basic search functionality
4. Create user interaction tracking
5. Develop trending content algorithms
6. Implement recommendation data collection
7. Create initial recommendation algorithms
8. Develop category and tag system
9. Implement personalization features
10. Optimize search relevance and performance
11. Develop content similarity algorithms
12. Implement recommendation serving infrastructure
13. Create A/B testing framework
14. Develop advanced recommendation algorithms
15. Implement search analytics and optimization

## 10. Success Criteria

- Search query response time < 200ms for 95th percentile
- Recommendation generation < 500ms for personalized recommendations
- Search relevance score > 0.8 (measured via user feedback)
- Recommendation click-through rate > 15%
- Trending content engagement > 25% higher than non-trending
- User retention improvement of > 10% with personalized recommendations
- System scales to handle 1M+ content items with minimal performance degradation
- Support for 100+ concurrent search queries per second 