//! Credit enforcement module for content generation
//!
//! Handles credit checking and consumption for AI-generated content

use crate::error::ServiceError;
use spin_sdk::pg::{Connection, ParameterValue};
use spin_sdk::outbound_http::{self, Method as HttpMethod, Request as OutboundRequest};
use spin_sdk::variables;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

//=============================================================================
// Credit Cost Configuration
//=============================================================================

/// Calculate estimated credit cost for generation
pub fn estimate_generation_cost(generation_type: &str, estimated_words: i32) -> i32 {
    match generation_type {
        "outline" => {
            // Outlines are typically 200-500 words, flat cost
            50
        },
        "chapter" => {
            // Chapters vary widely, estimate based on typical length
            // Average chapter: 2000-3000 words
            // Cost: 1 credit per 10 words = 200-300 credits
            (estimated_words as f32 * 0.1) as i32
        },
        "enhance" => {
            // Enhancement is iterative, charge based on content length
            // Cost: 1 credit per 20 words (cheaper than generation)
            (estimated_words as f32 * 0.05) as i32
        },
        _ => {
            // Default: 1 credit per 10 words
            (estimated_words as f32 * 0.1) as i32
        }
    }
}

//=============================================================================
// Credit Balance Checking via Subscription Service
//=============================================================================

#[derive(Debug, Deserialize)]
struct CreditBalanceResponse {
    balance: i32,
}

#[derive(Debug, Deserialize)]
struct CheckCreditsResponse {
    has_sufficient_credits: bool,
}

/// Check if user has sufficient credits by calling subscription service
pub fn check_user_credits(user_id: &Uuid, required_amount: i32) -> Result<bool, ServiceError> {
    let subscription_url = variables::get("subscription_service_url")
        .unwrap_or_else(|_| "http://subscription-service:3105".to_string());

    let request_body = serde_json::json!({
        "required_amount": required_amount
    });

    let request = OutboundRequest::builder()
        .method(HttpMethod::Post)
        .uri(format!("{}/credits/check", subscription_url))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id.to_string())
        .body(serde_json::to_vec(&request_body).unwrap())
        .build();

    let response = outbound_http::send(request)
        .map_err(|e| ServiceError::Internal(format!("Failed to check credits: {}", e)))?;

    if response.status().as_u16() != 200 {
        return Err(ServiceError::Internal("Credit check failed".into()));
    }

    let check_response: CheckCreditsResponse = serde_json::from_slice(response.body())
        .map_err(|e| ServiceError::Internal(format!("Failed to parse credit check response: {}", e)))?;

    Ok(check_response.has_sufficient_credits)
}

/// Get user's current credit balance
pub fn get_user_balance(user_id: &Uuid) -> Result<i32, ServiceError> {
    let subscription_url = variables::get("subscription_service_url")
        .unwrap_or_else(|_| "http://subscription-service:3105".to_string());

    let request = OutboundRequest::builder()
        .method(HttpMethod::Get)
        .uri(format!("{}/credits/balance", subscription_url))
        .header("X-User-Id", user_id.to_string())
        .build();

    let response = outbound_http::send(request)
        .map_err(|e| ServiceError::Internal(format!("Failed to get balance: {}", e)))?;

    if response.status().as_u16() != 200 {
        return Ok(0); // If user has no credit record, assume 0 balance
    }

    let balance_response: CreditBalanceResponse = serde_json::from_slice(response.body())
        .map_err(|e| ServiceError::Internal(format!("Failed to parse balance response: {}", e)))?;

    Ok(balance_response.balance)
}

//=============================================================================
// Credit Consumption
//=============================================================================

/// Consume credits for generation job
pub fn consume_credits_for_generation(
    user_id: &Uuid,
    amount: i32,
    job_id: &Uuid,
    job_type: &str,
) -> Result<bool, ServiceError> {
    let subscription_url = variables::get("subscription_service_url")
        .unwrap_or_else(|_| "http://subscription-service:3105".to_string());

    let request_body = serde_json::json!({
        "amount": amount,
        "reason": format!("{} generation", job_type),
        "reference_id": job_id.to_string(),
        "reference_type": "generation_job"
    });

    let request = OutboundRequest::builder()
        .method(HttpMethod::Post)
        .uri(format!("{}/credits/consume", subscription_url))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id.to_string())
        .body(serde_json::to_vec(&request_body).unwrap())
        .build();

    let response = outbound_http::send(request)
        .map_err(|e| ServiceError::Internal(format!("Failed to consume credits: {}", e)))?;

    Ok(response.status().as_u16() == 200)
}

//=============================================================================
// Database Credit Tracking
//=============================================================================

/// Record credit cost in generation_jobs table
pub fn record_job_credit_cost(
    conn: &Connection,
    job_id: &Uuid,
    credit_cost: i32,
) -> Result<(), ServiceError> {
    let query = "UPDATE content.generation_jobs
                 SET credits_cost = $1,
                     credits_charged = true
                 WHERE id = $2";

    let params = [
        ParameterValue::Int32(credit_cost),
        ParameterValue::Str(job_id.to_string()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Failed to record credit cost: {}", e)))?;

    Ok(())
}

/// Update book with credits used
pub fn update_book_credits_used(
    conn: &Connection,
    book_id: &Uuid,
    additional_credits: i32,
) -> Result<(), ServiceError> {
    let query = "UPDATE content.books
                 SET credits_used = COALESCE(credits_used, 0) + $1
                 WHERE id = $2";

    let params = [
        ParameterValue::Int32(additional_credits),
        ParameterValue::Str(book_id.to_string()),
    ];

    conn.execute(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Failed to update book credits: {}", e)))?;

    Ok(())
}

//=============================================================================
// Credit Enforcement Helper
//=============================================================================

/// Full credit enforcement workflow for generation request
/// Returns the credit cost that was reserved
pub fn enforce_credits_for_generation(
    conn: &Connection,
    user_id: &Uuid,
    job_id: &Uuid,
    book_id: &Uuid,
    generation_type: &str,
    estimated_words: i32,
) -> Result<i32, ServiceError> {
    // Calculate estimated cost
    let credit_cost = estimate_generation_cost(generation_type, estimated_words);

    // Check if user has sufficient credits
    let has_credits = check_user_credits(user_id, credit_cost)?;

    if !has_credits {
        let current_balance = get_user_balance(user_id)?;
        return Err(ServiceError::PaymentRequired(format!(
            "Insufficient credits. Required: {}, Available: {}",
            credit_cost, current_balance
        )));
    }

    // Consume the credits
    let consumed = consume_credits_for_generation(user_id, credit_cost, job_id, generation_type)?;

    if !consumed {
        return Err(ServiceError::PaymentRequired("Failed to consume credits".into()));
    }

    // Record cost in database
    record_job_credit_cost(conn, job_id, credit_cost)?;
    update_book_credits_used(conn, book_id, credit_cost)?;

    Ok(credit_cost)
}
