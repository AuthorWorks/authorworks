use axum::{
    extract::State,
    http::HeaderValue,
    routing::{get, post},
    Json, Router,
};
use qdrant_client::{client::QdrantClient, qdrant::{CreateCollection, Distance, VectorParams, UpsertPoints, PointStruct, SearchPoints}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    client: Arc<QdrantClient>,
}

#[derive(Debug, Deserialize)]
struct EnsureCollectionReq {
    collection: String,
    vector_size: u64,
    #[serde(default = "default_cosine")] distance: String,
}

fn default_cosine() -> String { "Cosine".to_string() }

#[derive(Debug, Deserialize)]
struct UpsertReq {
    collection: String,
    points: Vec<PointIn>,
}

#[derive(Debug, Deserialize)]
struct PointIn { id: String, vector: Vec<f32>, payload: Option<Value> }

#[derive(Debug, Deserialize)]
struct SearchReq {
    collection: String,
    vector: Vec<f32>,
    #[serde(default = "default_limit")] limit: u64,
}

fn default_limit() -> u64 { 5 }

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "authorworks_discovery_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let qdrant_url = std::env::var("QDRANT_GRPC_URL").unwrap_or_else(|_| "http://qdrant:6334".to_string());
    let client = QdrantClient::from_url(&qdrant_url).build().expect("qdrant client");
    let state = AppState { client: Arc::new(client) };

    let mut app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/discovery/ensure", post(ensure_collection))
        .route("/api/v1/discovery/upsert", post(upsert_points))
        .route("/api/v1/discovery/search", post(search_points));

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
        .unwrap_or(3007);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("invalid HOST/PORT");
    tracing::info!("AuthorWorks Discovery Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> { Json(json!({"service":"AuthorWorks Discovery Service","version":"0.1.0","status":"running"})) }
async fn health_check() -> Json<Value> { Json(json!({"status":"healthy","timestamp": chrono::Utc::now().to_rfc3339()})) }

async fn ensure_collection(State(state): State<AppState>, Json(req): Json<EnsureCollectionReq>) -> Json<Value> {
    let dist = match req.distance.as_str() { "Euclid" => Distance::Euclid, "Dot" => Distance::Dot, _ => Distance::Cosine };
    let params = VectorParams { size: req.vector_size, distance: dist as i32, ..Default::default() };
    let create = CreateCollection { collection_name: req.collection.clone(), vectors_config: Some(params.into()), ..Default::default() };
    let res = state.client.create_collection(&create).await;
    match res { Ok(_) => Json(json!({"status":"ok"})), Err(e) => Json(json!({"status":"error","error": e.to_string()})) }
}

async fn upsert_points(State(state): State<AppState>, Json(req): Json<UpsertReq>) -> Json<Value> {
    let points: Vec<PointStruct> = req.points.into_iter().map(|p| PointStruct::new(p.id, p.vector, p.payload)).collect();
    let up = UpsertPoints { collection_name: req.collection, points: Some(points.into()), ..Default::default() };
    let res = state.client.upsert_points(&up).await;
    match res { Ok(r) => Json(json!({"status":"ok","result": r.status()})), Err(e) => Json(json!({"status":"error","error": e.to_string()})) }
}

async fn search_points(State(state): State<AppState>, Json(req): Json<SearchReq>) -> Json<Value> {
    let search = SearchPoints { collection_name: req.collection, vector: req.vector, limit: req.limit, with_payload: Some(true.into()), ..Default::default() };
    let res = state.client.search_points(&search).await;
    match res { Ok(r) => Json(json!({"status":"ok","result": r.result})), Err(e) => Json(json!({"status":"error","error": e.to_string()})) }
}

