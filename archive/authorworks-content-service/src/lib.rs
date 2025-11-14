use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use anyhow::Result;

/// A simple Spin HTTP component for content service.
#[http_component]
fn handle_content_service(req: Request) -> Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();
    
    println!("Handling {:?} request to {}", method, path);
    
    let response = match (method, path) {
        (Method::Get, "/health") => {
            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(r#"{"status":"healthy","service":"content"}"#)
                .build()
        }
        (Method::Get, "/") => {
            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(r#"{"message":"AuthorWorks Content Service","version":"0.1.0"}"#)
                .build()
        }
        (Method::Get, "/api/content") => {
            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(r#"{"content":[],"total":0}"#)
                .build()
        }
        (Method::Post, "/api/content") => {
            Response::builder()
                .status(201)
                .header("content-type", "application/json")
                .body(r#"{"message":"Content created successfully"}"#)
                .build()
        }
        _ => {
            Response::builder()
                .status(404)
                .header("content-type", "application/json")
                .body(r#"{"error":"Not found"}"#)
                .build()
        }
    };
    
    Ok(response)
}