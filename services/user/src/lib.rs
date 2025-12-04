//! AuthorWorks User Service
//! 
//! Handles authentication, user profiles, and integrates with Logto for OAuth/SSO.
//! 
//! ## Endpoints
//! - POST /auth/register - Register new user
//! - POST /auth/login - Login with email/password
//! - POST /auth/logout - Logout and invalidate session
//! - POST /auth/refresh - Refresh access token
//! - GET /auth/callback - OAuth callback from Logto
//! - GET /users/me - Get current user profile
//! - PUT /users/me - Update current user profile
//! - GET /users/:id - Get public profile
//! - GET /health - Health check

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::variables;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

mod auth;
mod models;
mod handlers;
mod error;

use error::ServiceError;
use handlers::*;

/// Main HTTP component handler
#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();
    
    // Route to appropriate handler
    let result = match (method, path) {
        // Health check
        (Method::Get, "/health") => health_handler(),
        (Method::Get, "/") => service_info_handler(),
        
        // Authentication endpoints
        (Method::Post, "/auth/register") => register_handler(&req),
        (Method::Post, "/auth/login") => login_handler(&req),
        (Method::Post, "/auth/logout") => logout_handler(&req),
        (Method::Post, "/auth/refresh") => refresh_handler(&req),
        (Method::Get, "/auth/callback") => oauth_callback_handler(&req),
        (Method::Get, "/auth/logto/authorize") => logto_authorize_handler(&req),
        
        // User profile endpoints
        (Method::Get, "/users/me") => get_current_user_handler(&req),
        (Method::Put, "/users/me") => update_current_user_handler(&req),
        (Method::Get, path) if path.starts_with("/users/") => get_user_handler(&req, path),
        
        // Preferences
        (Method::Get, "/preferences") => get_preferences_handler(&req),
        (Method::Put, path) if path.starts_with("/preferences/") => set_preference_handler(&req, path),
        
        // CORS preflight
        (Method::Options, _) => cors_preflight_handler(),
        
        // Not found
        _ => Err(ServiceError::NotFound(format!("Route not found: {} {}", method, path))),
    };
    
    // Convert result to response
    match result {
        Ok(response) => Ok(response),
        Err(e) => Ok(e.into_response()),
    }
}

//=============================================================================
// Handler Implementations
//=============================================================================

fn health_handler() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "status": "healthy",
        "service": "user-service",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info_handler() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks User Service",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "User authentication and profile management with Logto SSO",
        "endpoints": {
            "auth": [
                "POST /auth/register",
                "POST /auth/login",
                "POST /auth/logout",
                "POST /auth/refresh",
                "GET /auth/callback",
                "GET /auth/logto/authorize"
            ],
            "users": [
                "GET /users/me",
                "PUT /users/me",
                "GET /users/:id"
            ],
            "preferences": [
                "GET /preferences",
                "PUT /preferences/:key"
            ],
            "health": [
                "GET /health"
            ]
        }
    }))
}

fn cors_preflight_handler() -> Result<Response, ServiceError> {
    Ok(Response::builder()
        .status(204)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .header("Access-Control-Max-Age", "86400")
        .body(())
        .build())
}

//=============================================================================
// Auth Handlers
//=============================================================================

fn register_handler(req: &Request) -> Result<Response, ServiceError> {
    let body: models::RegisterRequest = parse_json_body(req)?;
    
    // Validate input
    if body.email.is_empty() || !body.email.contains('@') {
        return Err(ServiceError::BadRequest("Invalid email address".into()));
    }
    if body.password.len() < 8 {
        return Err(ServiceError::BadRequest("Password must be at least 8 characters".into()));
    }
    if body.username.len() < 3 {
        return Err(ServiceError::BadRequest("Username must be at least 3 characters".into()));
    }
    
    // Create user (in production, this would store in database)
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let password_hash = auth::hash_password(&body.password)?;
    
    let user = models::User {
        id: user_id,
        email: body.email.clone(),
        username: body.username.clone(),
        display_name: body.display_name.unwrap_or(body.username.clone()),
        password_hash: Some(password_hash),
        auth_provider: Some(models::AuthProvider::Local),
        auth_provider_user_id: None,
        profile: models::Profile::default(),
        roles: vec!["user".to_string()],
        status: models::UserStatus::Unverified,
        created_at: now,
        updated_at: now,
        last_login_at: None,
    };
    
    // Generate tokens
    let tokens = auth::generate_tokens(&user)?;
    
    json_response(201, serde_json::json!({
        "user": models::PublicUser::from(&user),
        "tokens": tokens,
        "message": "Registration successful. Please verify your email."
    }))
}

fn login_handler(req: &Request) -> Result<Response, ServiceError> {
    let body: models::LoginRequest = parse_json_body(req)?;
    
    // In production, fetch user from database
    // For now, return a mock response demonstrating the flow
    
    // Validate credentials would happen here
    if body.email.is_empty() || body.password.is_empty() {
        return Err(ServiceError::Unauthorized("Invalid credentials".into()));
    }
    
    // Mock user for demonstration
    let user = models::User {
        id: Uuid::new_v4(),
        email: body.email.clone(),
        username: body.email.split('@').next().unwrap_or("user").to_string(),
        display_name: "Demo User".to_string(),
        password_hash: None,
        auth_provider: Some(models::AuthProvider::Local),
        auth_provider_user_id: None,
        profile: models::Profile::default(),
        roles: vec!["user".to_string()],
        status: models::UserStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: Some(Utc::now()),
    };
    
    let tokens = auth::generate_tokens(&user)?;
    
    json_response(200, serde_json::json!({
        "user": models::PublicUser::from(&user),
        "tokens": tokens
    }))
}

fn logout_handler(req: &Request) -> Result<Response, ServiceError> {
    // In production, invalidate the refresh token in database/redis
    let _token = extract_bearer_token(req)?;
    
    json_response(200, serde_json::json!({
        "message": "Logged out successfully"
    }))
}

fn refresh_handler(req: &Request) -> Result<Response, ServiceError> {
    let body: models::RefreshRequest = parse_json_body(req)?;
    
    // Validate refresh token and generate new access token
    let claims = auth::validate_refresh_token(&body.refresh_token)?;
    
    // Generate new access token
    let access_token = auth::generate_access_token(&claims.sub, &claims.roles)?;
    
    json_response(200, serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": 3600
    }))
}

fn oauth_callback_handler(req: &Request) -> Result<Response, ServiceError> {
    // Parse query parameters
    let query = req.query();
    let code = get_query_param(query, "code")
        .ok_or_else(|| ServiceError::BadRequest("Missing authorization code".into()))?;
    let state = get_query_param(query, "state");
    
    // Exchange code for tokens with Logto
    let logto_endpoint = get_config("LOGTO_ENDPOINT")?;
    let client_id = get_config("LOGTO_CLIENT_ID")?;
    let client_secret = get_config("LOGTO_CLIENT_SECRET")?;
    let redirect_uri = get_config("LOGTO_REDIRECT_URI")?;
    
    // In production, make HTTP request to Logto token endpoint
    // For now, return success with placeholder
    json_response(200, serde_json::json!({
        "message": "OAuth callback received",
        "code": code,
        "state": state,
        "note": "Token exchange would happen here with Logto"
    }))
}

fn logto_authorize_handler(_req: &Request) -> Result<Response, ServiceError> {
    let logto_endpoint = get_config("LOGTO_ENDPOINT").unwrap_or_else(|_| "https://auth.authorworks.leopaska.xyz".to_string());
    let client_id = get_config("LOGTO_CLIENT_ID").unwrap_or_else(|_| "authorworks-app".to_string());
    let redirect_uri = get_config("LOGTO_REDIRECT_URI").unwrap_or_else(|_| "https://authorworks.leopaska.xyz/auth/callback".to_string());
    
    let state = Uuid::new_v4().to_string();
    let authorize_url = format!(
        "{}/oidc/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email&state={}",
        logto_endpoint, client_id, urlencoding(&redirect_uri), state
    );
    
    // Redirect to Logto
    Ok(Response::builder()
        .status(302)
        .header("Location", &authorize_url)
        .header("Content-Type", "text/html")
        .body(format!("Redirecting to <a href=\"{}\">Logto</a>...", authorize_url))
        .build())
}

//=============================================================================
// User Handlers
//=============================================================================

fn get_current_user_handler(req: &Request) -> Result<Response, ServiceError> {
    let token = extract_bearer_token(req)?;
    let claims = auth::validate_access_token(&token)?;
    
    // In production, fetch user from database
    let user = models::User {
        id: Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()),
        email: "user@example.com".to_string(),
        username: "user".to_string(),
        display_name: "Demo User".to_string(),
        password_hash: None,
        auth_provider: Some(models::AuthProvider::Local),
        auth_provider_user_id: None,
        profile: models::Profile::default(),
        roles: claims.roles.clone(),
        status: models::UserStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: Some(Utc::now()),
    };
    
    json_response(200, serde_json::json!({
        "user": models::PublicUser::from(&user)
    }))
}

fn update_current_user_handler(req: &Request) -> Result<Response, ServiceError> {
    let token = extract_bearer_token(req)?;
    let claims = auth::validate_access_token(&token)?;
    let body: models::UpdateProfileRequest = parse_json_body(req)?;
    
    // In production, update user in database
    json_response(200, serde_json::json!({
        "message": "Profile updated successfully",
        "user_id": claims.sub,
        "updated_fields": body
    }))
}

fn get_user_handler(_req: &Request, path: &str) -> Result<Response, ServiceError> {
    let user_id = path.strip_prefix("/users/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;
    
    // Validate UUID
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| ServiceError::BadRequest("Invalid user ID format".into()))?;
    
    // In production, fetch public user profile from database
    let public_user = models::PublicUser {
        id: uuid,
        username: "demo_user".to_string(),
        display_name: "Demo User".to_string(),
        profile: models::Profile::default(),
        created_at: Utc::now(),
    };
    
    json_response(200, serde_json::json!({
        "user": public_user
    }))
}

//=============================================================================
// Preferences Handlers
//=============================================================================

fn get_preferences_handler(req: &Request) -> Result<Response, ServiceError> {
    let token = extract_bearer_token(req)?;
    let claims = auth::validate_access_token(&token)?;
    
    // In production, fetch from database
    let preferences: HashMap<String, serde_json::Value> = HashMap::new();
    
    json_response(200, serde_json::json!({
        "user_id": claims.sub,
        "preferences": preferences
    }))
}

fn set_preference_handler(req: &Request, path: &str) -> Result<Response, ServiceError> {
    let token = extract_bearer_token(req)?;
    let claims = auth::validate_access_token(&token)?;
    
    let key = path.strip_prefix("/preferences/")
        .ok_or_else(|| ServiceError::BadRequest("Invalid path".into()))?;
    
    let body: serde_json::Value = parse_json_body(req)?;
    
    // In production, store in database
    json_response(200, serde_json::json!({
        "message": "Preference updated",
        "user_id": claims.sub,
        "key": key,
        "value": body
    }))
}

//=============================================================================
// Utility Functions
//=============================================================================

fn json_response<T: Serialize>(status: u16, body: T) -> Result<Response, ServiceError> {
    let json = serde_json::to_string(&body)
        .map_err(|e| ServiceError::Internal(format!("JSON serialization error: {}", e)))?;
    
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(json)
        .build())
}

fn parse_json_body<T: for<'de> Deserialize<'de>>(req: &Request) -> Result<T, ServiceError> {
    let body = req.body();
    serde_json::from_slice(body)
        .map_err(|e| ServiceError::BadRequest(format!("Invalid JSON: {}", e)))
}

fn extract_bearer_token(req: &Request) -> Result<String, ServiceError> {
    let auth_header = req.header("Authorization")
        .and_then(|h| h.as_str())
        .ok_or_else(|| ServiceError::Unauthorized("Missing Authorization header".into()))?;
    
    auth_header
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| ServiceError::Unauthorized("Invalid Authorization header format".into()))
}

fn get_query_param(query: &str, key: &str) -> Option<String> {
    query.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next()?;
            if k == key { Some(v.to_string()) } else { None }
        })
        .next()
}

fn get_config(key: &str) -> Result<String, ServiceError> {
    variables::get(key)
        .map_err(|_| ServiceError::Internal(format!("Missing config: {}", key)))
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
