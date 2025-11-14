use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let response = match req.path() {
        "/health" => Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(r#"{"status":"healthy"}"#)
            .build(),
        "/" => Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(r#"{"service":"SERVICE_NAME","version":"0.1.0"}"#)
            .build(),
        _ => Response::builder()
            .status(404)
            .body(r#"{"error":"Not found"}"#)
            .build(),
    };
    Ok(response)
}
