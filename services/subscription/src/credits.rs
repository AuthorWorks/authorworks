

//! Credit System Module
//!
//! Handles credit packages, purchases, consumption, and balance management.

use crate::error::ServiceError;
use crate::models::*;
use serde::{Deserialize, Serialize};
use spin_sdk::http::{Request, Response};
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use uuid::Uuid;
use chrono::Utc;

//=============================================================================
// Models
//=============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditPackage {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub credit_amount: i32,
    pub price_cents: i32,
    pub stripe_price_id: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditBalance {
    pub user_id: Uuid,
    pub balance: i32,
    pub total_purchased: i32,
    pub total_consumed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i32,
    pub balance_after: i32,
    pub transaction_type: String,
    pub reason: Option<String>,
    pub reference_id: Option<Uuid>,
    pub reference_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub package_id: Option<Uuid>,
    pub credit_amount: i32,
    pub price_cents: i32,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_checkout_session_id: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseCreditsRequest {
    pub package_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ConsumeCreditsRequest {
    pub amount: i32,
    pub reason: String,
    pub reference_id: Option<Uuid>,
    pub reference_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutSessionResponse {
    pub session_id: String,
    pub checkout_url: String,
}

//=============================================================================
// Credit Package Endpoints
//=============================================================================

/// GET /credits/packages - List available credit packages
pub fn list_credit_packages(conn: &Connection) -> Result<Response, ServiceError> {
    let query = "SELECT id, name, description, credit_amount, price_cents,
                 stripe_price_id, is_active, sort_order, metadata
                 FROM subscriptions.credit_packages
                 WHERE is_active = true
                 ORDER BY sort_order ASC";

    let result = conn.query(query, &[])
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    let mut packages = Vec::new();
    for row in result.rows() {
        packages.push(CreditPackage {
            id: Uuid::parse_str(row.get::<&str>("id").unwrap()).unwrap(),
            name: row.get::<&str>("name").unwrap().to_string(),
            description: row.get::<Option<&str>>("description").unwrap().map(|s| s.to_string()),
            credit_amount: row.get::<i32>("credit_amount").unwrap(),
            price_cents: row.get::<i32>("price_cents").unwrap(),
            stripe_price_id: row.get::<Option<&str>>("stripe_price_id").unwrap().map(|s| s.to_string()),
            is_active: row.get::<bool>("is_active").unwrap(),
            sort_order: row.get::<i32>("sort_order").unwrap(),
            metadata: serde_json::from_str(row.get::<&str>("metadata").unwrap_or("{}")).unwrap_or(serde_json::json!({})),
        });
    }

    crate::json_response(200, serde_json::json!({
        "packages": packages
    }))
}

//=============================================================================
// Credit Balance Endpoints
//=============================================================================

/// GET /credits/balance - Get user's credit balance
pub fn get_credit_balance(conn: &Connection, user_id: Uuid) -> Result<Response, ServiceError> {
    let query = "SELECT user_id, balance, total_purchased, total_consumed
                 FROM subscriptions.credits
                 WHERE user_id = $1";

    let params = [ParameterValue::Uuid(user_id.as_bytes())];
    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    if result.rows().is_empty() {
        // No record yet, return zero balance
        return crate::json_response(200, serde_json::json!({
            "user_id": user_id,
            "balance": 0,
            "total_purchased": 0,
            "total_consumed": 0
        }));
    }

    let row = &result.rows()[0];
    let balance = CreditBalance {
        user_id,
        balance: row.get::<i32>("balance").unwrap(),
        total_purchased: row.get::<i32>("total_purchased").unwrap(),
        total_consumed: row.get::<i32>("total_consumed").unwrap(),
    };

    crate::json_response(200, serde_json::json!(balance))
}

/// GET /credits/history - Get user's credit transaction history
pub fn get_credit_history(conn: &Connection, user_id: Uuid, limit: i32) -> Result<Response, ServiceError> {
    let query = "SELECT id, user_id, amount, balance_after, transaction_type,
                 reason, reference_id, reference_type, created_at
                 FROM subscriptions.credit_transactions
                 WHERE user_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2";

    let params = [
        ParameterValue::Uuid(user_id.as_bytes()),
        ParameterValue::Int32(limit),
    ];
    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    let mut transactions = Vec::new();
    for row in result.rows() {
        transactions.push(CreditTransaction {
            id: Uuid::parse_str(row.get::<&str>("id").unwrap()).unwrap(),
            user_id,
            amount: row.get::<i32>("amount").unwrap(),
            balance_after: row.get::<i32>("balance_after").unwrap(),
            transaction_type: row.get::<&str>("transaction_type").unwrap().to_string(),
            reason: row.get::<Option<&str>>("reason").unwrap().map(|s| s.to_string()),
            reference_id: row.get::<Option<&str>>("reference_id").unwrap()
                .and_then(|s| Uuid::parse_str(s).ok()),
            reference_type: row.get::<Option<&str>>("reference_type").unwrap().map(|s| s.to_string()),
            created_at: row.get::<&str>("created_at").unwrap().to_string(),
        });
    }

    crate::json_response(200, serde_json::json!({
        "transactions": transactions,
        "count": transactions.len()
    }))
}

//=============================================================================
// Credit Purchase Endpoints
//=============================================================================

/// POST /credits/checkout - Create Stripe checkout session for credit purchase
pub fn create_credit_checkout_session(
    conn: &Connection,
    user_id: Uuid,
    package_id: Uuid,
) -> Result<Response, ServiceError> {
    // Get package details
    let query = "SELECT id, name, credit_amount, price_cents, stripe_price_id
                 FROM subscriptions.credit_packages
                 WHERE id = $1 AND is_active = true";

    let params = [ParameterValue::Uuid(package_id.as_bytes())];
    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    if result.rows().is_empty() {
        return Err(ServiceError::NotFound("Credit package not found".into()));
    }

    let row = &result.rows()[0];
    let package_name = row.get::<&str>("name").unwrap();
    let credit_amount = row.get::<i32>("credit_amount").unwrap();
    let price_cents = row.get::<i32>("price_cents").unwrap();
    let stripe_price_id = row.get::<Option<&str>>("stripe_price_id").unwrap();

    // Create pending order
    let order_id = Uuid::new_v4();
    let insert_query = "INSERT INTO subscriptions.credit_orders
                       (id, user_id, package_id, credit_amount, price_cents, status)
                       VALUES ($1, $2, $3, $4, $5, 'pending')";

    let insert_params = [
        ParameterValue::Uuid(order_id.as_bytes()),
        ParameterValue::Uuid(user_id.as_bytes()),
        ParameterValue::Uuid(package_id.as_bytes()),
        ParameterValue::Int32(credit_amount),
        ParameterValue::Int32(price_cents),
    ];
    conn.execute(insert_query, &insert_params)
        .map_err(|e| ServiceError::Internal(format!("Failed to create order: {}", e)))?;

    // Call Stripe API to create checkout session
    let stripe_secret_key = variables::get("stripe_secret_key")
        .map_err(|_| ServiceError::Internal("STRIPE_SECRET_KEY not configured".into()))?;

    // Note: In a real implementation, you would call Stripe's API here
    // For now, we'll return a mock response
    let session_id = format!("cs_test_{}", order_id);
    let checkout_url = format!("https://checkout.stripe.com/pay/{}", session_id);

    // Update order with session ID
    let update_query = "UPDATE subscriptions.credit_orders
                       SET stripe_checkout_session_id = $1
                       WHERE id = $2";
    let update_params = [
        ParameterValue::Str(session_id.clone()),
        ParameterValue::Uuid(order_id.as_bytes()),
    ];
    conn.execute(update_query, &update_params)
        .map_err(|e| ServiceError::Internal(format!("Failed to update order: {}", e)))?;

    crate::json_response(200, serde_json::json!({
        "order_id": order_id,
        "session_id": session_id,
        "checkout_url": checkout_url,
        "package": {
            "name": package_name,
            "credits": credit_amount,
            "price_cents": price_cents
        }
    }))
}

/// POST /credits/webhook - Handle Stripe webhook for credit purchase completion
pub fn handle_credit_purchase_webhook(
    conn: &Connection,
    session_id: &str,
    payment_intent_id: &str,
    status: &str,
) -> Result<(), ServiceError> {
    if status != "complete" && status != "paid" {
        return Ok(());
    }

    // Find order by session ID
    let query = "SELECT id, user_id, credit_amount FROM subscriptions.credit_orders
                 WHERE stripe_checkout_session_id = $1 AND status = 'pending'";
    let params = [ParameterValue::Str(session_id.to_string())];
    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    if result.rows().is_empty() {
        return Ok(()); // Order already processed or not found
    }

    let row = &result.rows()[0];
    let order_id = Uuid::parse_str(row.get::<&str>("id").unwrap()).unwrap();
    let user_id = Uuid::parse_str(row.get::<&str>("user_id").unwrap()).unwrap();
    let credit_amount = row.get::<i32>("credit_amount").unwrap();

    // Update order status
    let update_order = "UPDATE subscriptions.credit_orders
                       SET status = 'completed',
                           stripe_payment_intent_id = $1,
                           completed_at = NOW()
                       WHERE id = $2";
    conn.execute(update_order, &[
        ParameterValue::Str(payment_intent_id.to_string()),
        ParameterValue::Uuid(order_id.as_bytes()),
    ]).map_err(|e| ServiceError::Internal(format!("Failed to update order: {}", e)))?;

    // Add credits to user account using stored procedure
    let add_credits = "SELECT subscriptions.add_credits($1, $2, 'purchase', 'Credit purchase', $3, 'order')";
    conn.query(add_credits, &[
        ParameterValue::Uuid(user_id.as_bytes()),
        ParameterValue::Int32(credit_amount),
        ParameterValue::Uuid(order_id.as_bytes()),
    ]).map_err(|e| ServiceError::Internal(format!("Failed to add credits: {}", e)))?;

    Ok(())
}

//=============================================================================
// Credit Consumption (for content service)
//=============================================================================

/// Check if user has sufficient credits
pub fn check_sufficient_credits(
    conn: &Connection,
    user_id: Uuid,
    required_amount: i32,
) -> Result<bool, ServiceError> {
    let query = "SELECT subscriptions.has_sufficient_credits($1, $2)";
    let params = [
        ParameterValue::Uuid(user_id.as_bytes()),
        ParameterValue::Int32(required_amount),
    ];

    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    if result.rows().is_empty() {
        return Ok(false);
    }

    let has_credits = result.rows()[0].get::<bool>("has_sufficient_credits").unwrap();
    Ok(has_credits)
}

/// Consume credits for content generation
pub fn consume_credits(
    conn: &Connection,
    user_id: Uuid,
    amount: i32,
    reason: &str,
    reference_id: Option<Uuid>,
    reference_type: Option<&str>,
) -> Result<bool, ServiceError> {
    let query = "SELECT subscriptions.consume_credits($1, $2, $3, $4, $5)";
    let params = [
        ParameterValue::Uuid(user_id.as_bytes()),
        ParameterValue::Int32(amount),
        ParameterValue::Str(reason.to_string()),
        ParameterValue::Uuid(reference_id.unwrap_or(Uuid::nil()).as_bytes()),
        ParameterValue::Str(reference_type.unwrap_or("").to_string()),
    ];

    let result = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Database query failed: {}", e)))?;

    if result.rows().is_empty() {
        return Ok(false);
    }

    let consumed = result.rows()[0].get::<bool>("consume_credits").unwrap();
    Ok(consumed)
}

//=============================================================================
// Credit Cost Configuration
//=============================================================================

/// Calculate credit cost for content generation
pub fn calculate_generation_cost(word_count: i32, generation_type: &str) -> i32 {
    match generation_type {
        "outline" => (word_count as f32 * 0.1) as i32,  // 1 credit per 10 words
        "chapter" => (word_count as f32 * 0.2) as i32,  // 1 credit per 5 words
        "scene" => (word_count as f32 * 0.2) as i32,    // 1 credit per 5 words
        "character" => 10,                               // Flat 10 credits
        "dialogue" => (word_count as f32 * 0.15) as i32, // 1 credit per ~7 words
        _ => word_count / 5,                            // Default: 1 credit per 5 words
    }
}
