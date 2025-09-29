use axum::{
    extract::{Query, State},
    http::HeaderValue,
    routing::{get, post},
    Json, Router,
};
use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    redis_client: Arc<redis::Client>,
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    stream: String,
    message: Value,
}

#[derive(Debug, Deserialize)]
struct GroupCreate {
    stream: String,
    group: String,
}

#[derive(Debug, Deserialize)]
struct ConsumeParams {
    stream: String,
    group: String,
    consumer: String,
    #[serde(default = "default_count")] 
    count: usize,
    #[serde(default = "default_block")] 
    block_ms: usize,
}

fn default_count() -> usize { 10 }
fn default_block() -> usize { 1000 }

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "authorworks_messaging_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Redis client
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    let state = AppState { redis_client: Arc::new(client) };

    let mut app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/messages/publish", post(publish))
        .route("/api/v1/messages/groups", post(create_group))
        .route("/api/v1/messages/consume", get(consume));

    let cors = match std::env::var("ALLOWED_ORIGINS") {
        Ok(val) if !val.trim().is_empty() => {
            let origins: Vec<HeaderValue> = val
                .split(',')
                .filter_map(|o| HeaderValue::from_str(o.trim()).ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(AllowMethods::any())
                .allow_headers(AllowHeaders::any())
        }
        _ => CorsLayer::permissive(),
    };
    app = app.layer(cors).with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("SERVICE_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3006);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("invalid HOST/PORT");
    tracing::info!("AuthorWorks Messaging Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({
        "service": "AuthorWorks Messaging Service",
        "version": "0.1.0",
        "status": "running"
    }))
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn publish(State(state): State<AppState>, Json(body): Json<PublishRequest>) -> Json<Value> {
    let payload = serde_json::to_string(&body.message).unwrap_or("{}".to_string());
    let mut conn = state.redis_client.get_async_connection().await.unwrap();
    // XADD stream * data payload
    let id: RedisResult<String> = redis::cmd("XADD")
        .arg(&body.stream)
        .arg("*")
        .arg("data")
        .arg(&payload)
        .query_async(&mut conn)
        .await;
    match id {
        Ok(id) => Json(json!({"status":"ok","id": id})),
        Err(e) => Json(json!({"status":"error","error": e.to_string()})),
    }
}

async fn create_group(State(state): State<AppState>, Json(body): Json<GroupCreate>) -> Json<Value> {
    let mut conn = state.redis_client.get_async_connection().await.unwrap();
    // XGROUP CREATE <stream> <group> $ MKSTREAM
    let res: RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&body.stream)
        .arg(&body.group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(&mut conn)
        .await;
    match res {
        Ok(_) => Json(json!({"status":"ok"})),
        Err(e) => Json(json!({"status":"error","error": e.to_string()})),
    }
}

async fn consume(State(state): State<AppState>, Query(q): Query<ConsumeParams>) -> Json<Value> {
    let mut conn = state.redis_client.get_async_connection().await.unwrap();
    // XREADGROUP GROUP <group> <consumer> COUNT <count> BLOCK <block_ms> STREAMS <stream> >
    let res: RedisResult<Value> = redis::cmd("XREADGROUP")
        .arg("GROUP").arg(&q.group).arg(&q.consumer)
        .arg("COUNT").arg(q.count)
        .arg("BLOCK").arg(q.block_ms)
        .arg("STREAMS").arg(&q.stream).arg(">")
        .query_async(&mut conn)
        .await
        .map(|raw: redis::Value| redis_value_to_json(raw));

    match res {
        Ok(v) => Json(json!({"status":"ok","messages": v})),
        Err(e) => Json(json!({"status":"error","error": e.to_string()})),
    }
}

fn redis_value_to_json(v: redis::Value) -> Value {
    match v {
        redis::Value::Nil => Value::Null,
        redis::Value::Int(i) => Value::from(i),
        redis::Value::Data(d) => Value::from(String::from_utf8_lossy(&d).to_string()),
        redis::Value::Bulk(b) => Value::from(
            b.into_iter().map(redis_value_to_json).collect::<Vec<_>>()
        ),
        redis::Value::Status(s) => Value::from(s),
        redis::Value::Okay => Value::from("OK"),
        redis::Value::Map(m) => {
            let obj = m.into_iter()
                .map(|(k, v)| (match k { redis::Value::Data(d) => String::from_utf8_lossy(&d).to_string(), _ => format!("{:?}", k) }, redis_value_to_json(v)))
                .collect::<serde_json::Map<_, _>>();
            Value::Object(obj)
        }
    }
}

