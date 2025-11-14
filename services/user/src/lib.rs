use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// User authentication and profile management service
#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let response = match req.path() {
        "/health" => Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(r#"{"status":"healthy","service":"user-service"}"#)
            .build(),
        "/" => Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(r#"{"service":"User Service","version":"0.1.0","endpoints":["/health","/api/auth/register","/api/auth/login","/api/users/profile"]}"#)
            .build(),
        _ => Response::builder()
            .status(404)
            .header("content-type", "application/json")
            .body(r#"{"error":"Not found"}"#)
            .build(),
    };

    Ok(response)
}
