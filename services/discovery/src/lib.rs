//! AuthorWorks Discovery Service
//!
//! Provides search, recommendations, and content discovery via Elasticsearch.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - GET /search - Full-text search across content
//! - GET /search/books - Search books
//! - GET /search/chapters - Search chapters
//! - GET /search/authors - Search authors
//! - POST /index/book - Index a book (internal)
//! - POST /index/chapter - Index a chapter (internal)
//! - DELETE /index/book/:id - Remove book from index
//! - GET /recommendations - Get personalized recommendations
//! - GET /trending - Get trending content
//! - GET /similar/:book_id - Get similar books

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use spin_sdk::outbound_http;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

mod models;
mod error;

use error::ServiceError;
use models::*;

#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();

    let result = match (method, path) {
        // Health
        (Method::Get, "/health") => health_handler(),
        (Method::Get, "/") => service_info(),

        // Search
        (Method::Get, "/search") => search_all(&req),
        (Method::Get, "/search/books") => search_books(&req),
        (Method::Get, "/search/chapters") => search_chapters(&req),
        (Method::Get, "/search/authors") => search_authors(&req),

        // Indexing (internal)
        (Method::Post, "/index/book") => index_book(&req),
        (Method::Post, "/index/chapter") => index_chapter(&req),
        (Method::Delete, path) if path.starts_with("/index/book/") => delete_book_index(&req, path),

        // Discovery
        (Method::Get, "/recommendations") => get_recommendations(&req),
        (Method::Get, "/trending") => get_trending(&req),
        (Method::Get, path) if path.starts_with("/similar/") => get_similar(&req, path),

        // CORS
        (Method::Options, _) => cors_preflight(),

        _ => Err(ServiceError::NotFound(format!("Route not found: {} {}", method, path))),
    };

    match result {
        Ok(response) => Ok(response),
        Err(e) => Ok(e.into_response()),
    }
}

//=============================================================================
// Configuration
//=============================================================================

fn get_db_connection() -> Result<Connection, ServiceError> {
    let url = variables::get("database_url")
        .map_err(|_| ServiceError::Internal("DATABASE_URL not configured".into()))?;
    Connection::open(&url)
        .map_err(|e| ServiceError::Internal(format!("Database connection failed: {}", e)))
}

fn get_elasticsearch_url() -> Result<String, ServiceError> {
    variables::get("elasticsearch_url")
        .or_else(|_| Ok("http://elasticsearch:9200".into()))
}

fn get_user_id(req: &Request) -> Result<Uuid, ServiceError> {
    let user_id = req.header("X-User-Id")
        .and_then(|h| h.as_str())
        .ok_or_else(|| ServiceError::Unauthorized("Missing user ID".into()))?;
    
    Uuid::parse_str(user_id)
        .map_err(|_| ServiceError::Unauthorized("Invalid user ID".into()))
}

fn get_optional_user_id(req: &Request) -> Option<Uuid> {
    req.header("X-User-Id")
        .and_then(|h| h.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

//=============================================================================
// Health & Info
//=============================================================================

fn health_handler() -> Result<Response, ServiceError> {
    let db_status = match get_db_connection() {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let es_url = get_elasticsearch_url().unwrap_or_default();
    let es_status = check_elasticsearch_health(&es_url);

    json_response(200, serde_json::json!({
        "status": "healthy",
        "service": "discovery-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "elasticsearch": es_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn check_elasticsearch_health(es_url: &str) -> &'static str {
    let url = format!("{}/_cluster/health", es_url);
    let request = outbound_http::Request::builder()
        .method("GET")
        .uri(&url)
        .build();

    match outbound_http::send(request) {
        Ok(resp) if resp.status() < 400 => "connected",
        _ => "disconnected",
    }
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Discovery Service",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["full-text-search", "recommendations", "similar-content"]
    }))
}

fn cors_preflight() -> Result<Response, ServiceError> {
    Ok(Response::builder()
        .status(204)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-User-Id")
        .header("Access-Control-Max-Age", "86400")
        .body(())
        .build())
}

//=============================================================================
// Search
//=============================================================================

fn search_all(req: &Request) -> Result<Response, ServiceError> {
    let query = get_query_param(req, "q")
        .ok_or_else(|| ServiceError::BadRequest("Query parameter 'q' is required".into()))?;
    let from = get_query_param(req, "from").and_then(|s| s.parse().ok()).unwrap_or(0);
    let size = get_query_param(req, "size").and_then(|s| s.parse().ok()).unwrap_or(20);

    let es_url = get_elasticsearch_url()?;
    
    // Multi-index search
    let search_body = serde_json::json!({
        "query": {
            "multi_match": {
                "query": query,
                "fields": ["title^3", "description^2", "content", "author_name", "genre"],
                "type": "best_fields",
                "fuzziness": "AUTO"
            }
        },
        "highlight": {
            "fields": {
                "title": {},
                "description": {},
                "content": { "fragment_size": 150 }
            }
        },
        "from": from,
        "size": size
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-*/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());
    let total = response.get("hits").and_then(|h| h.get("total")).and_then(|t| t.get("value")).and_then(|v| v.as_i64()).unwrap_or(0);

    let results: Vec<SearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            let index = hit.get("_index")?.as_str()?;
            let highlight = hit.get("highlight");
            
            Some(SearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                result_type: if index.contains("books") { "book" } else if index.contains("chapters") { "chapter" } else { "author" }.to_string(),
                title: source.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                description: source.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                highlight: highlight.map(|h| h.clone()),
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "results": results,
        "total": total,
        "from": from,
        "size": size
    }))
}

fn search_books(req: &Request) -> Result<Response, ServiceError> {
    let query = get_query_param(req, "q")
        .ok_or_else(|| ServiceError::BadRequest("Query parameter 'q' is required".into()))?;
    let genre = get_query_param(req, "genre");
    let status = get_query_param(req, "status");
    let from = get_query_param(req, "from").and_then(|s| s.parse().ok()).unwrap_or(0);
    let size = get_query_param(req, "size").and_then(|s| s.parse().ok()).unwrap_or(20);

    let es_url = get_elasticsearch_url()?;

    // Build query with filters
    let mut must = vec![serde_json::json!({
        "multi_match": {
            "query": query,
            "fields": ["title^3", "description^2", "genre", "author_name"],
            "fuzziness": "AUTO"
        }
    })];

    let mut filter = Vec::new();
    if let Some(g) = genre {
        filter.push(serde_json::json!({"term": {"genre": g}}));
    }
    if let Some(s) = status {
        filter.push(serde_json::json!({"term": {"status": s}}));
    }

    let search_body = serde_json::json!({
        "query": {
            "bool": {
                "must": must,
                "filter": filter
            }
        },
        "highlight": {
            "fields": {
                "title": {},
                "description": {}
            }
        },
        "sort": [
            "_score",
            {"updated_at": "desc"}
        ],
        "from": from,
        "size": size
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-books/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());
    let total = response.get("hits").and_then(|h| h.get("total")).and_then(|t| t.get("value")).and_then(|v| v.as_i64()).unwrap_or(0);

    let books: Vec<BookSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            Some(BookSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                title: source.get("title")?.as_str()?.to_string(),
                description: source.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                author_name: source.get("author_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                genre: source.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string()),
                status: source.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string(),
                cover_url: source.get("cover_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                word_count: source.get("word_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "books": books,
        "total": total,
        "from": from,
        "size": size
    }))
}

fn search_chapters(req: &Request) -> Result<Response, ServiceError> {
    let query = get_query_param(req, "q")
        .ok_or_else(|| ServiceError::BadRequest("Query parameter 'q' is required".into()))?;
    let book_id = get_query_param(req, "book_id");
    let from = get_query_param(req, "from").and_then(|s| s.parse().ok()).unwrap_or(0);
    let size = get_query_param(req, "size").and_then(|s| s.parse().ok()).unwrap_or(20);

    let es_url = get_elasticsearch_url()?;

    let mut filter = Vec::new();
    if let Some(bid) = book_id {
        filter.push(serde_json::json!({"term": {"book_id": bid}}));
    }

    let search_body = serde_json::json!({
        "query": {
            "bool": {
                "must": [{
                    "multi_match": {
                        "query": query,
                        "fields": ["title^2", "content"],
                        "fuzziness": "AUTO"
                    }
                }],
                "filter": filter
            }
        },
        "highlight": {
            "fields": {
                "title": {},
                "content": { "fragment_size": 200, "number_of_fragments": 3 }
            }
        },
        "from": from,
        "size": size
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-chapters/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());
    let total = response.get("hits").and_then(|h| h.get("total")).and_then(|t| t.get("value")).and_then(|v| v.as_i64()).unwrap_or(0);

    let chapters: Vec<ChapterSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            let highlight = hit.get("highlight");
            Some(ChapterSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                book_id: source.get("book_id")?.as_str()?.to_string(),
                title: source.get("title")?.as_str()?.to_string(),
                chapter_number: source.get("chapter_number").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                highlight: highlight.and_then(|h| h.get("content")).and_then(|c| c.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "chapters": chapters,
        "total": total,
        "from": from,
        "size": size
    }))
}

fn search_authors(req: &Request) -> Result<Response, ServiceError> {
    let query = get_query_param(req, "q")
        .ok_or_else(|| ServiceError::BadRequest("Query parameter 'q' is required".into()))?;
    let from = get_query_param(req, "from").and_then(|s| s.parse().ok()).unwrap_or(0);
    let size = get_query_param(req, "size").and_then(|s| s.parse().ok()).unwrap_or(20);

    let es_url = get_elasticsearch_url()?;

    let search_body = serde_json::json!({
        "query": {
            "multi_match": {
                "query": query,
                "fields": ["name^3", "bio", "genres"],
                "fuzziness": "AUTO"
            }
        },
        "from": from,
        "size": size
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-authors/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());
    let total = response.get("hits").and_then(|h| h.get("total")).and_then(|t| t.get("value")).and_then(|v| v.as_i64()).unwrap_or(0);

    let authors: Vec<AuthorSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            Some(AuthorSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                name: source.get("name")?.as_str()?.to_string(),
                bio: source.get("bio").and_then(|v| v.as_str()).map(|s| s.to_string()),
                avatar_url: source.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                book_count: source.get("book_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "authors": authors,
        "total": total,
        "from": from,
        "size": size
    }))
}

//=============================================================================
// Indexing
//=============================================================================

fn index_book(req: &Request) -> Result<Response, ServiceError> {
    let body: IndexBookRequest = parse_json_body(req)?;
    let es_url = get_elasticsearch_url()?;

    let doc = serde_json::json!({
        "id": body.id,
        "title": body.title,
        "description": body.description,
        "author_id": body.author_id,
        "author_name": body.author_name,
        "genre": body.genre,
        "status": body.status,
        "cover_url": body.cover_url,
        "word_count": body.word_count,
        "created_at": body.created_at,
        "updated_at": body.updated_at
    });

    elasticsearch_request(&es_url, "PUT", &format!("/authorworks-books/_doc/{}", body.id), &doc)?;

    json_response(200, serde_json::json!({"indexed": true}))
}

fn index_chapter(req: &Request) -> Result<Response, ServiceError> {
    let body: IndexChapterRequest = parse_json_body(req)?;
    let es_url = get_elasticsearch_url()?;

    let doc = serde_json::json!({
        "id": body.id,
        "book_id": body.book_id,
        "title": body.title,
        "content": body.content,
        "chapter_number": body.chapter_number,
        "word_count": body.word_count,
        "created_at": body.created_at,
        "updated_at": body.updated_at
    });

    elasticsearch_request(&es_url, "PUT", &format!("/authorworks-chapters/_doc/{}", body.id), &doc)?;

    json_response(200, serde_json::json!({"indexed": true}))
}

fn delete_book_index(_req: &Request, path: &str) -> Result<Response, ServiceError> {
    let book_id = path.strip_prefix("/index/book/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    let es_url = get_elasticsearch_url()?;

    // Delete book document
    elasticsearch_request(&es_url, "DELETE", &format!("/authorworks-books/_doc/{}", book_id), &serde_json::json!({}))?;

    // Delete associated chapters
    let delete_query = serde_json::json!({
        "query": {
            "term": {"book_id": book_id}
        }
    });
    elasticsearch_request(&es_url, "POST", "/authorworks-chapters/_delete_by_query", &delete_query)?;

    json_response(200, serde_json::json!({"deleted": true}))
}

//=============================================================================
// Discovery
//=============================================================================

fn get_recommendations(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_optional_user_id(req);
    let conn = get_db_connection()?;
    let es_url = get_elasticsearch_url()?;

    // Get user's reading history and preferences
    let genres = if let Some(uid) = user_id {
        let query = "SELECT DISTINCT b.genre FROM content.books b
                     JOIN discovery.reading_history h ON b.id = h.book_id
                     WHERE h.user_id = $1 AND b.genre IS NOT NULL
                     LIMIT 5";
        let params = [ParameterValue::Str(uid.to_string())];
        let rows = conn.query(query, &params)
            .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;
        
        rows.rows.iter()
            .filter_map(|r| String::decode(&r[0]).ok())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    // Build recommendation query
    let search_body = if !genres.is_empty() {
        serde_json::json!({
            "query": {
                "bool": {
                    "should": genres.iter().map(|g| {
                        serde_json::json!({"term": {"genre": g}})
                    }).collect::<Vec<_>>(),
                    "filter": [
                        {"term": {"status": "published"}}
                    ],
                    "minimum_should_match": 1
                }
            },
            "sort": [
                {"_score": "desc"},
                {"word_count": "desc"}
            ],
            "size": 20
        })
    } else {
        // Generic recommendations for anonymous users
        serde_json::json!({
            "query": {
                "bool": {
                    "filter": [
                        {"term": {"status": "published"}}
                    ]
                }
            },
            "sort": [
                {"updated_at": "desc"}
            ],
            "size": 20
        })
    };

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-books/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());

    let recommendations: Vec<BookSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            Some(BookSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                title: source.get("title")?.as_str()?.to_string(),
                description: source.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                author_name: source.get("author_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                genre: source.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string()),
                status: source.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string(),
                cover_url: source.get("cover_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                word_count: source.get("word_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "recommendations": recommendations,
        "personalized": user_id.is_some()
    }))
}

fn get_trending(_req: &Request) -> Result<Response, ServiceError> {
    let conn = get_db_connection()?;
    let es_url = get_elasticsearch_url()?;

    // Get books with most activity in last 7 days
    let query = "SELECT book_id, COUNT(*) as activity
                 FROM discovery.reading_history
                 WHERE created_at > NOW() - INTERVAL '7 days'
                 GROUP BY book_id
                 ORDER BY activity DESC
                 LIMIT 20";

    let rows = conn.query(query, &[])
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let book_ids: Vec<String> = rows.rows.iter()
        .filter_map(|r| String::decode(&r[0]).ok())
        .collect();

    if book_ids.is_empty() {
        return json_response(200, serde_json::json!({
            "trending": [],
            "period": "7d"
        }));
    }

    // Fetch book details from Elasticsearch
    let search_body = serde_json::json!({
        "query": {
            "ids": {"values": book_ids}
        },
        "size": 20
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-books/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());

    let trending: Vec<BookSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            Some(BookSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                title: source.get("title")?.as_str()?.to_string(),
                description: source.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                author_name: source.get("author_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                genre: source.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string()),
                status: source.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string(),
                cover_url: source.get("cover_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                word_count: source.get("word_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                score: 0.0,
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "trending": trending,
        "period": "7d"
    }))
}

fn get_similar(_req: &Request, path: &str) -> Result<Response, ServiceError> {
    let book_id = path.strip_prefix("/similar/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;

    let es_url = get_elasticsearch_url()?;

    // Use More Like This query
    let search_body = serde_json::json!({
        "query": {
            "more_like_this": {
                "fields": ["title", "description", "genre"],
                "like": [
                    {"_index": "authorworks-books", "_id": book_id}
                ],
                "min_term_freq": 1,
                "min_doc_freq": 1,
                "max_query_terms": 25
            }
        },
        "size": 10
    });

    let response = elasticsearch_request(&es_url, "GET", "/authorworks-books/_search", &search_body)?;
    
    let hits = response.get("hits").and_then(|h| h.get("hits")).and_then(|h| h.as_array());

    let similar: Vec<BookSearchResult> = hits.map(|arr| {
        arr.iter().filter_map(|hit| {
            let source = hit.get("_source")?;
            Some(BookSearchResult {
                id: source.get("id")?.as_str()?.to_string(),
                title: source.get("title")?.as_str()?.to_string(),
                description: source.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                author_name: source.get("author_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                genre: source.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string()),
                status: source.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string(),
                cover_url: source.get("cover_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                word_count: source.get("word_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                score: hit.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0),
            })
        }).collect()
    }).unwrap_or_default();

    json_response(200, serde_json::json!({
        "similar": similar,
        "book_id": book_id
    }))
}

//=============================================================================
// Elasticsearch Helpers
//=============================================================================

fn elasticsearch_request(es_url: &str, method: &str, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, ServiceError> {
    let url = format!("{}{}", es_url, path);
    
    let request = outbound_http::Request::builder()
        .method(method)
        .uri(&url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .build();

    let response = outbound_http::send(request)
        .map_err(|e| ServiceError::Internal(format!("Elasticsearch request failed: {}", e)))?;

    if response.status() >= 400 {
        return Err(ServiceError::Internal(format!(
            "Elasticsearch error: {} - {}",
            response.status(),
            String::from_utf8_lossy(response.body())
        )));
    }

    serde_json::from_slice(response.body())
        .map_err(|e| ServiceError::Internal(format!("Failed to parse Elasticsearch response: {}", e)))
}

//=============================================================================
// Helper Functions
//=============================================================================

fn get_query_param(req: &Request, name: &str) -> Option<String> {
    let path = req.path();
    let query_start = path.find('?')?;
    let query_str = &path[query_start + 1..];
    
    for pair in query_str.split('&') {
        let mut kv = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
            if key == name {
                return Some(urlencoded_decode(value));
            }
        }
    }
    None
}

fn urlencoded_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                    result.push(byte as char);
                    continue;
                }
            }
        } else if c == '+' {
            result.push(' ');
            continue;
        }
        result.push(c);
    }
    result
}

fn json_response<T: Serialize>(status: u16, body: T) -> Result<Response, ServiceError> {
    let json = serde_json::to_string(&body)
        .map_err(|e| ServiceError::Internal(format!("JSON error: {}", e)))?;

    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(json)
        .build())
}

fn parse_json_body<T: for<'de> Deserialize<'de>>(req: &Request) -> Result<T, ServiceError> {
    serde_json::from_slice(req.body())
        .map_err(|e| ServiceError::BadRequest(format!("Invalid JSON: {}", e)))
}
