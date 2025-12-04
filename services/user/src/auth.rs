//! Authentication utilities for the User Service
//! 
//! Handles JWT token generation/validation, password hashing, and Logto integration.

use crate::models::{User, AuthTokens};
use crate::error::ServiceError;
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use spin_sdk::variables;

type HmacSha256 = Hmac<Sha256>;

//=============================================================================
// JWT Claims
//=============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // Subject (user ID)
    pub email: String,         // User email
    pub roles: Vec<String>,    // User roles
    pub exp: usize,            // Expiration time
    pub iat: usize,            // Issued at
    pub iss: String,           // Issuer
    pub aud: String,           // Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<String>, // Auth provider (local, logto, etc.)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub roles: Vec<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub token_type: String,
}

//=============================================================================
// Token Generation
//=============================================================================

/// Generate access and refresh tokens for a user
pub fn generate_tokens(user: &User) -> Result<AuthTokens, ServiceError> {
    let access_token = generate_access_token(&user.id.to_string(), &user.roles)?;
    let refresh_token = generate_refresh_token(&user.id.to_string(), &user.roles)?;
    
    Ok(AuthTokens {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
    })
}

/// Generate an access token (short-lived)
pub fn generate_access_token(user_id: &str, roles: &[String]) -> Result<String, ServiceError> {
    let secret = get_jwt_secret()?;
    let now = Utc::now();
    let expiry = now + Duration::hours(1);
    
    let claims = Claims {
        sub: user_id.to_string(),
        email: String::new(), // Would be populated from user data
        roles: roles.to_vec(),
        exp: expiry.timestamp() as usize,
        iat: now.timestamp() as usize,
        iss: "authorworks".to_string(),
        aud: "authorworks-api".to_string(),
        auth_provider: None,
    };
    
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ServiceError::Internal(format!("Token generation failed: {}", e)))
}

/// Generate a refresh token (long-lived)
pub fn generate_refresh_token(user_id: &str, roles: &[String]) -> Result<String, ServiceError> {
    let secret = get_jwt_secret()?;
    let now = Utc::now();
    let expiry = now + Duration::days(30);
    
    let claims = RefreshClaims {
        sub: user_id.to_string(),
        roles: roles.to_vec(),
        exp: expiry.timestamp() as usize,
        iat: now.timestamp() as usize,
        iss: "authorworks".to_string(),
        token_type: "refresh".to_string(),
    };
    
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ServiceError::Internal(format!("Refresh token generation failed: {}", e)))
}

//=============================================================================
// Token Validation
//=============================================================================

/// Validate an access token and return claims
pub fn validate_access_token(token: &str) -> Result<Claims, ServiceError> {
    let secret = get_jwt_secret()?;
    
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["authorworks"]);
    validation.set_audience(&["authorworks-api"]);
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            ServiceError::Unauthorized("Token expired".into())
        }
        jsonwebtoken::errors::ErrorKind::InvalidToken => {
            ServiceError::Unauthorized("Invalid token".into())
        }
        _ => ServiceError::Unauthorized(format!("Token validation failed: {}", e)),
    })?;
    
    Ok(token_data.claims)
}

/// Validate a refresh token and return claims
pub fn validate_refresh_token(token: &str) -> Result<RefreshClaims, ServiceError> {
    let secret = get_jwt_secret()?;
    
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["authorworks"]);
    
    let token_data = decode::<RefreshClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| ServiceError::Unauthorized(format!("Invalid refresh token: {}", e)))?;
    
    if token_data.claims.token_type != "refresh" {
        return Err(ServiceError::Unauthorized("Invalid token type".into()));
    }
    
    Ok(token_data.claims)
}

//=============================================================================
// Password Hashing
//=============================================================================

/// Hash a password using SHA-256 with salt
/// Note: In production, use argon2 or bcrypt. SHA-256 is used here for WASM compatibility.
pub fn hash_password(password: &str) -> Result<String, ServiceError> {
    let salt = get_password_salt()?;
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, ServiceError> {
    let computed_hash = hash_password(password)?;
    Ok(computed_hash == hash)
}

//=============================================================================
// Logto Integration
//=============================================================================

/// Logto configuration
pub struct LogtoConfig {
    pub endpoint: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl LogtoConfig {
    pub fn from_env() -> Result<Self, ServiceError> {
        Ok(LogtoConfig {
            endpoint: get_env_or_default("LOGTO_ENDPOINT", "https://auth.authorworks.leopaska.xyz"),
            client_id: get_env_or_default("LOGTO_CLIENT_ID", "authorworks-app"),
            client_secret: variables::get("LOGTO_CLIENT_SECRET")
                .map_err(|_| ServiceError::Internal("Missing LOGTO_CLIENT_SECRET".into()))?,
            redirect_uri: get_env_or_default("LOGTO_REDIRECT_URI", "https://authorworks.leopaska.xyz/auth/callback"),
        })
    }
    
    /// Build the authorization URL for Logto
    pub fn authorization_url(&self, state: &str) -> String {
        format!(
            "{}/oidc/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email&state={}",
            self.endpoint,
            self.client_id,
            urlencoding(&self.redirect_uri),
            state
        )
    }
    
    /// Build the token endpoint URL
    pub fn token_url(&self) -> String {
        format!("{}/oidc/token", self.endpoint)
    }
    
    /// Build the userinfo endpoint URL
    pub fn userinfo_url(&self) -> String {
        format!("{}/oidc/me", self.endpoint)
    }
}

//=============================================================================
// Utility Functions
//=============================================================================

fn get_jwt_secret() -> Result<String, ServiceError> {
    variables::get("JWT_SECRET")
        .or_else(|_| Ok("authorworks-dev-secret-change-in-production".to_string()))
}

fn get_password_salt() -> Result<String, ServiceError> {
    variables::get("PASSWORD_SALT")
        .or_else(|_| Ok("authorworks-salt".to_string()))
}

fn get_env_or_default(key: &str, default: &str) -> String {
    variables::get(key).unwrap_or_else(|_| default.to_string())
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

//=============================================================================
// HMAC Utilities for Webhook Verification
//=============================================================================

/// Verify an HMAC signature (useful for Logto webhooks)
pub fn verify_hmac_signature(payload: &[u8], signature: &str, secret: &str) -> Result<bool, ServiceError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| ServiceError::Internal(format!("HMAC error: {}", e)))?;
    mac.update(payload);
    
    let expected = hex::decode(signature)
        .map_err(|_| ServiceError::BadRequest("Invalid signature format".into()))?;
    
    Ok(mac.verify_slice(&expected).is_ok())
}

