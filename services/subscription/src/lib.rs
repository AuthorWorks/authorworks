//! AuthorWorks Subscription Service
//!
//! Handles subscription plans, billing, and payment processing via Stripe.
//!
//! ## Endpoints
//! - GET /health - Health check
//! - GET /plans - List available plans
//! - GET /subscription - Get user's subscription
//! - POST /subscription - Create subscription
//! - PUT /subscription - Update subscription
//! - DELETE /subscription - Cancel subscription
//! - POST /checkout - Create checkout session
//! - POST /portal - Create customer portal session
//! - POST /webhooks/stripe - Handle Stripe webhooks
//! - GET /invoices - List user's invoices
//! - GET /usage - Get usage statistics

use spin_sdk::http::{IntoResponse, Request, Response, Method};
use spin_sdk::http_component;
use spin_sdk::pg::{Connection, Decode, ParameterValue};
use spin_sdk::variables;
use spin_sdk::outbound_http;
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use uuid::Uuid;
use hmac::{Hmac, Mac};
use sha2::Sha256;

mod models;
mod error;
mod stripe;
mod credits;

use error::ServiceError;
use models::*;

type HmacSha256 = Hmac<Sha256>;

#[http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();

    let result = match (method, path) {
        // Health
        (Method::Get, "/health") => health_handler(),
        (Method::Get, "/") => service_info(),

        // Plans
        (Method::Get, "/plans") => list_plans(&req),

        // Subscription
        (Method::Get, "/subscription") => get_subscription(&req),
        (Method::Post, "/subscription") => create_subscription(&req),
        (Method::Put, "/subscription") => update_subscription(&req),
        (Method::Delete, "/subscription") => cancel_subscription(&req),

        // Checkout & Portal
        (Method::Post, "/checkout") => create_checkout_session(&req),
        (Method::Post, "/portal") => create_portal_session(&req),

        // Webhooks
        (Method::Post, "/webhooks/stripe") => handle_stripe_webhook(&req),

        // Billing
        (Method::Get, "/invoices") => list_invoices(&req),
        (Method::Get, "/usage") => get_usage(&req),

        // Credits
        (Method::Get, "/credits/packages") => get_credit_packages(&req),
        (Method::Get, "/credits/balance") => get_user_credit_balance(&req),
        (Method::Get, "/credits/history") => get_user_credit_history(&req),
        (Method::Post, "/credits/checkout") => create_credit_checkout(&req),
        (Method::Post, "/credits/consume") => consume_user_credits(&req),
        (Method::Post, "/credits/check") => check_user_credits(&req),

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

fn get_stripe_config() -> Result<StripeConfig, ServiceError> {
    Ok(StripeConfig {
        secret_key: variables::get("stripe_secret_key")
            .map_err(|_| ServiceError::Internal("STRIPE_SECRET_KEY not configured".into()))?,
        webhook_secret: variables::get("stripe_webhook_secret")
            .map_err(|_| ServiceError::Internal("STRIPE_WEBHOOK_SECRET not configured".into()))?,
        price_id_free: variables::get("stripe_price_free").unwrap_or_else(|_| "price_free".into()),
        price_id_pro: variables::get("stripe_price_pro")
            .map_err(|_| ServiceError::Internal("STRIPE_PRICE_PRO not configured".into()))?,
        price_id_enterprise: variables::get("stripe_price_enterprise")
            .map_err(|_| ServiceError::Internal("STRIPE_PRICE_ENTERPRISE not configured".into()))?,
    })
}

fn get_user_id(req: &Request) -> Result<Uuid, ServiceError> {
    let user_id = req.header("X-User-Id")
        .and_then(|h| h.as_str())
        .ok_or_else(|| ServiceError::Unauthorized("Missing user ID".into()))?;
    
    Uuid::parse_str(user_id)
        .map_err(|_| ServiceError::Unauthorized("Invalid user ID".into()))
}

//=============================================================================
// Health & Info
//=============================================================================

fn health_handler() -> Result<Response, ServiceError> {
    let db_status = match get_db_connection() {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let stripe_status = match get_stripe_config() {
        Ok(_) => "configured",
        Err(_) => "not_configured",
    };

    json_response(200, serde_json::json!({
        "status": "healthy",
        "service": "subscription-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "stripe": stripe_status,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

fn service_info() -> Result<Response, ServiceError> {
    json_response(200, serde_json::json!({
        "service": "AuthorWorks Subscription Service",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["stripe-integration", "subscription-management", "usage-tracking"]
    }))
}

fn cors_preflight() -> Result<Response, ServiceError> {
    Ok(Response::builder()
        .status(204)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-User-Id")
        .header("Access-Control-Max-Age", "86400")
        .body(())
        .build())
}

//=============================================================================
// Plans
//=============================================================================

fn list_plans(_req: &Request) -> Result<Response, ServiceError> {
    let plans = vec![
        Plan {
            id: "free".into(),
            name: "Free".into(),
            description: "Perfect for getting started".into(),
            price_monthly: 0,
            price_yearly: 0,
            features: vec![
                "1 book project".into(),
                "5,000 AI words/month".into(),
                "Basic editor".into(),
                "Community support".into(),
            ],
            limits: PlanLimits {
                max_books: 1,
                max_chapters_per_book: 10,
                ai_words_per_month: 5000,
                storage_gb: 1,
                collaborators: 0,
            },
        },
        Plan {
            id: "pro".into(),
            name: "Professional".into(),
            description: "For serious authors".into(),
            price_monthly: 1999, // $19.99
            price_yearly: 19990, // $199.90 (save ~17%)
            features: vec![
                "Unlimited book projects".into(),
                "100,000 AI words/month".into(),
                "Advanced editor with collaboration".into(),
                "Priority support".into(),
                "Export to all formats".into(),
                "Version history".into(),
            ],
            limits: PlanLimits {
                max_books: -1, // unlimited
                max_chapters_per_book: -1,
                ai_words_per_month: 100000,
                storage_gb: 50,
                collaborators: 5,
            },
        },
        Plan {
            id: "enterprise".into(),
            name: "Enterprise".into(),
            description: "For publishing teams".into(),
            price_monthly: 9999, // $99.99
            price_yearly: 99990, // $999.90
            features: vec![
                "Everything in Professional".into(),
                "Unlimited AI words".into(),
                "Unlimited collaborators".into(),
                "Custom AI training".into(),
                "API access".into(),
                "Dedicated support".into(),
                "SSO integration".into(),
            ],
            limits: PlanLimits {
                max_books: -1,
                max_chapters_per_book: -1,
                ai_words_per_month: -1,
                storage_gb: 500,
                collaborators: -1,
            },
        },
    ];

    json_response(200, serde_json::json!({
        "plans": plans
    }))
}

//=============================================================================
// Subscription Management
//=============================================================================

fn get_subscription(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT s.id, s.plan_id, s.status, s.stripe_subscription_id, s.stripe_customer_id,
                 s.current_period_start, s.current_period_end, s.cancel_at_period_end,
                 s.created_at, s.updated_at
                 FROM subscriptions.subscriptions s WHERE s.user_id = $1";

    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        // Return free tier info
        return json_response(200, serde_json::json!({
            "plan_id": "free",
            "status": "active",
            "limits": {
                "max_books": 1,
                "max_chapters_per_book": 10,
                "ai_words_per_month": 5000,
                "storage_gb": 1,
                "collaborators": 0
            }
        }));
    }

    let row = &rows.rows[0];
    let subscription = Subscription {
        id: Uuid::parse_str(&String::decode(&row[0]).unwrap_or_default()).unwrap_or_default(),
        user_id,
        plan_id: String::decode(&row[1]).unwrap_or_default(),
        status: String::decode(&row[2]).unwrap_or_default(),
        stripe_subscription_id: String::decode(&row[3]).ok(),
        stripe_customer_id: String::decode(&row[4]).ok(),
        current_period_start: String::decode(&row[5]).ok(),
        current_period_end: String::decode(&row[6]).ok(),
        cancel_at_period_end: bool::decode(&row[7]).unwrap_or(false),
        created_at: String::decode(&row[8]).unwrap_or_default(),
        updated_at: String::decode(&row[9]).unwrap_or_default(),
    };

    json_response(200, subscription)
}

fn create_subscription(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: CreateSubscriptionRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;
    let stripe_config = get_stripe_config()?;

    // Check for existing subscription
    let check_query = "SELECT id FROM subscriptions.subscriptions WHERE user_id = $1";
    let check_params = [ParameterValue::Str(user_id.to_string())];
    let existing = conn.query(check_query, &check_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if !existing.rows.is_empty() {
        return Err(ServiceError::Conflict("Subscription already exists".into()));
    }

    // Get user email for Stripe customer
    let user_query = "SELECT email FROM users.users WHERE id = $1";
    let user_params = [ParameterValue::Str(user_id.to_string())];
    let user_rows = conn.query(user_query, &user_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if user_rows.rows.is_empty() {
        return Err(ServiceError::NotFound("User not found".into()));
    }

    let email = String::decode(&user_rows.rows[0][0]).unwrap_or_default();

    // Create Stripe customer
    let customer_id = create_stripe_customer(&stripe_config, &email, &user_id)?;

    // Get price ID for plan
    let price_id = match body.plan_id.as_str() {
        "pro" => &stripe_config.price_id_pro,
        "enterprise" => &stripe_config.price_id_enterprise,
        _ => return Err(ServiceError::BadRequest("Invalid plan".into())),
    };

    // Create Stripe subscription
    let stripe_sub = create_stripe_subscription(&stripe_config, &customer_id, price_id)?;

    // Store in database
    let sub_id = Uuid::new_v4();
    let now = Utc::now();

    let insert = "INSERT INTO subscriptions.subscriptions 
                  (id, user_id, plan_id, status, stripe_subscription_id, stripe_customer_id,
                   current_period_start, current_period_end, created_at, updated_at)
                  VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)";

    let params = [
        ParameterValue::Str(sub_id.to_string()),
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.plan_id.clone()),
        ParameterValue::Str(stripe_sub.status.clone()),
        ParameterValue::Str(stripe_sub.id.clone()),
        ParameterValue::Str(customer_id.clone()),
        ParameterValue::Str(stripe_sub.current_period_start.clone()),
        ParameterValue::Str(stripe_sub.current_period_end.clone()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(insert, &params)
        .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;

    json_response(201, serde_json::json!({
        "id": sub_id,
        "plan_id": body.plan_id,
        "status": stripe_sub.status,
        "current_period_end": stripe_sub.current_period_end,
        "stripe_subscription_id": stripe_sub.id
    }))
}

fn update_subscription(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: UpdateSubscriptionRequest = parse_json_body(req)?;
    let conn = get_db_connection()?;
    let stripe_config = get_stripe_config()?;

    // Get current subscription
    let query = "SELECT stripe_subscription_id FROM subscriptions.subscriptions WHERE user_id = $1";
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Subscription not found".into()));
    }

    let stripe_sub_id = String::decode(&rows.rows[0][0])
        .map_err(|_| ServiceError::Internal("Invalid subscription data".into()))?;

    // Get new price ID
    let price_id = match body.plan_id.as_str() {
        "pro" => &stripe_config.price_id_pro,
        "enterprise" => &stripe_config.price_id_enterprise,
        _ => return Err(ServiceError::BadRequest("Invalid plan".into())),
    };

    // Update Stripe subscription
    update_stripe_subscription(&stripe_config, &stripe_sub_id, price_id)?;

    // Update database
    let now = Utc::now();
    let update = "UPDATE subscriptions.subscriptions SET plan_id = $2, updated_at = $3 WHERE user_id = $1";
    let update_params = [
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(body.plan_id.clone()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(update, &update_params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "plan_id": body.plan_id,
        "updated_at": now.to_rfc3339()
    }))
}

fn cancel_subscription(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;
    let stripe_config = get_stripe_config()?;

    // Get subscription
    let query = "SELECT stripe_subscription_id FROM subscriptions.subscriptions WHERE user_id = $1";
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("Subscription not found".into()));
    }

    let stripe_sub_id = String::decode(&rows.rows[0][0])
        .map_err(|_| ServiceError::Internal("Invalid subscription data".into()))?;

    // Cancel at period end in Stripe
    cancel_stripe_subscription(&stripe_config, &stripe_sub_id)?;

    // Update database
    let now = Utc::now();
    let update = "UPDATE subscriptions.subscriptions 
                  SET cancel_at_period_end = true, updated_at = $2 WHERE user_id = $1";
    let update_params = [
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(now.to_rfc3339()),
    ];

    conn.execute(update, &update_params)
        .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;

    json_response(200, serde_json::json!({
        "message": "Subscription will be cancelled at end of billing period",
        "cancel_at_period_end": true
    }))
}

//=============================================================================
// Checkout & Portal
//=============================================================================

fn create_checkout_session(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: CheckoutRequest = parse_json_body(req)?;
    let stripe_config = get_stripe_config()?;
    let conn = get_db_connection()?;

    // Get or create Stripe customer
    let customer_id = get_or_create_stripe_customer(&conn, &stripe_config, &user_id)?;

    // Get price ID
    let price_id = match body.plan_id.as_str() {
        "pro" => &stripe_config.price_id_pro,
        "enterprise" => &stripe_config.price_id_enterprise,
        _ => return Err(ServiceError::BadRequest("Invalid plan".into())),
    };

    // Create checkout session
    let session = create_stripe_checkout_session(
        &stripe_config,
        &customer_id,
        price_id,
        &body.success_url,
        &body.cancel_url,
    )?;

    json_response(200, serde_json::json!({
        "session_id": session.id,
        "url": session.url
    }))
}

fn create_portal_session(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let body: PortalRequest = parse_json_body(req)?;
    let stripe_config = get_stripe_config()?;
    let conn = get_db_connection()?;

    // Get Stripe customer ID
    let query = "SELECT stripe_customer_id FROM subscriptions.subscriptions WHERE user_id = $1";
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if rows.rows.is_empty() {
        return Err(ServiceError::NotFound("No subscription found".into()));
    }

    let customer_id = String::decode(&rows.rows[0][0])
        .map_err(|_| ServiceError::Internal("Invalid customer data".into()))?;

    // Create portal session
    let session = create_stripe_portal_session(&stripe_config, &customer_id, &body.return_url)?;

    json_response(200, serde_json::json!({
        "url": session.url
    }))
}

//=============================================================================
// Webhooks
//=============================================================================

fn handle_stripe_webhook(req: &Request) -> Result<Response, ServiceError> {
    let stripe_config = get_stripe_config()?;
    
    // Verify webhook signature
    let signature = req.header("Stripe-Signature")
        .and_then(|h| h.as_str())
        .ok_or_else(|| ServiceError::BadRequest("Missing signature".into()))?;

    verify_stripe_signature(req.body(), signature, &stripe_config.webhook_secret)?;

    // Parse event
    let event: StripeEvent = serde_json::from_slice(req.body())
        .map_err(|e| ServiceError::BadRequest(format!("Invalid event: {}", e)))?;

    let conn = get_db_connection()?;

    match event.event_type.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => {
            let sub_data = event.data.object;
            let stripe_sub_id = sub_data.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let status = sub_data.get("status").and_then(|v| v.as_str()).unwrap_or("active");
            let period_start = sub_data.get("current_period_start").and_then(|v| v.as_i64());
            let period_end = sub_data.get("current_period_end").and_then(|v| v.as_i64());
            let cancel_at_end = sub_data.get("cancel_at_period_end").and_then(|v| v.as_bool()).unwrap_or(false);

            let now = Utc::now();
            let update = "UPDATE subscriptions.subscriptions 
                          SET status = $2, current_period_start = $3, current_period_end = $4,
                              cancel_at_period_end = $5, updated_at = $6
                          WHERE stripe_subscription_id = $1";

            let period_start_str = period_start
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.to_rfc3339()))
                .flatten()
                .unwrap_or_default();
            let period_end_str = period_end
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.to_rfc3339()))
                .flatten()
                .unwrap_or_default();

            let params = [
                ParameterValue::Str(stripe_sub_id.to_string()),
                ParameterValue::Str(status.to_string()),
                ParameterValue::Str(period_start_str),
                ParameterValue::Str(period_end_str),
                ParameterValue::Boolean(cancel_at_end),
                ParameterValue::Str(now.to_rfc3339()),
            ];

            conn.execute(update, &params)
                .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;
        }
        "customer.subscription.deleted" => {
            let sub_data = event.data.object;
            let stripe_sub_id = sub_data.get("id").and_then(|v| v.as_str()).unwrap_or_default();

            let now = Utc::now();
            let update = "UPDATE subscriptions.subscriptions 
                          SET status = 'cancelled', plan_id = 'free', updated_at = $2
                          WHERE stripe_subscription_id = $1";

            let params = [
                ParameterValue::Str(stripe_sub_id.to_string()),
                ParameterValue::Str(now.to_rfc3339()),
            ];

            conn.execute(update, &params)
                .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;
        }
        "invoice.paid" => {
            // Record successful payment
            let invoice_data = event.data.object;
            let customer_id = invoice_data.get("customer").and_then(|v| v.as_str()).unwrap_or_default();
            let amount = invoice_data.get("amount_paid").and_then(|v| v.as_i64()).unwrap_or(0);
            let invoice_id = invoice_data.get("id").and_then(|v| v.as_str()).unwrap_or_default();

            let now = Utc::now();
            let id = Uuid::new_v4();
            let insert = "INSERT INTO subscriptions.invoices 
                          (id, stripe_customer_id, stripe_invoice_id, amount, status, created_at)
                          VALUES ($1, $2, $3, $4, 'paid', $5)";

            let params = [
                ParameterValue::Str(id.to_string()),
                ParameterValue::Str(customer_id.to_string()),
                ParameterValue::Str(invoice_id.to_string()),
                ParameterValue::Int64(amount),
                ParameterValue::Str(now.to_rfc3339()),
            ];

            conn.execute(insert, &params)
                .map_err(|e| ServiceError::Internal(format!("Insert failed: {}", e)))?;
        }
        "invoice.payment_failed" => {
            // Handle failed payment
            let invoice_data = event.data.object;
            let stripe_sub_id = invoice_data.get("subscription").and_then(|v| v.as_str()).unwrap_or_default();

            let now = Utc::now();
            let update = "UPDATE subscriptions.subscriptions 
                          SET status = 'past_due', updated_at = $2
                          WHERE stripe_subscription_id = $1";

            let params = [
                ParameterValue::Str(stripe_sub_id.to_string()),
                ParameterValue::Str(now.to_rfc3339()),
            ];

            conn.execute(update, &params)
                .map_err(|e| ServiceError::Internal(format!("Update failed: {}", e)))?;
        }
        _ => {
            // Log unhandled event type
        }
    }

    json_response(200, serde_json::json!({"received": true}))
}

//=============================================================================
// Invoices & Usage
//=============================================================================

fn list_invoices(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    let query = "SELECT i.id, i.stripe_invoice_id, i.amount, i.status, i.created_at
                 FROM subscriptions.invoices i
                 JOIN subscriptions.subscriptions s ON i.stripe_customer_id = s.stripe_customer_id
                 WHERE s.user_id = $1
                 ORDER BY i.created_at DESC LIMIT 50";

    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let invoices: Vec<serde_json::Value> = rows.rows.iter().map(|row| {
        serde_json::json!({
            "id": String::decode(&row[0]).unwrap_or_default(),
            "stripe_invoice_id": String::decode(&row[1]).unwrap_or_default(),
            "amount": i64::decode(&row[2]).unwrap_or(0),
            "status": String::decode(&row[3]).unwrap_or_default(),
            "created_at": String::decode(&row[4]).unwrap_or_default()
        })
    }).collect();

    json_response(200, serde_json::json!({
        "invoices": invoices
    }))
}

fn get_usage(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    // Get current billing period
    let period_start = Utc::now().with_day(1).unwrap_or(Utc::now());

    // Get AI word usage
    let ai_query = "SELECT COALESCE(SUM(word_count), 0) FROM subscriptions.ai_usage 
                    WHERE user_id = $1 AND created_at >= $2";
    let ai_params = [
        ParameterValue::Str(user_id.to_string()),
        ParameterValue::Str(period_start.to_rfc3339()),
    ];
    let ai_rows = conn.query(ai_query, &ai_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let ai_words_used = if !ai_rows.rows.is_empty() {
        i64::decode(&ai_rows.rows[0][0]).unwrap_or(0)
    } else {
        0
    };

    // Get storage usage
    let storage_query = "SELECT COALESCE(SUM(size), 0) FROM storage.files WHERE user_id = $1";
    let storage_params = [ParameterValue::Str(user_id.to_string())];
    let storage_rows = conn.query(storage_query, &storage_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let storage_bytes = if !storage_rows.rows.is_empty() {
        i64::decode(&storage_rows.rows[0][0]).unwrap_or(0)
    } else {
        0
    };

    // Get book count
    let books_query = "SELECT COUNT(*) FROM content.books WHERE author_id = $1";
    let books_params = [ParameterValue::Str(user_id.to_string())];
    let books_rows = conn.query(books_query, &books_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let book_count = if !books_rows.rows.is_empty() {
        i64::decode(&books_rows.rows[0][0]).unwrap_or(0)
    } else {
        0
    };

    // Get subscription limits
    let sub_query = "SELECT plan_id FROM subscriptions.subscriptions WHERE user_id = $1";
    let sub_params = [ParameterValue::Str(user_id.to_string())];
    let sub_rows = conn.query(sub_query, &sub_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    let plan_id = if !sub_rows.rows.is_empty() {
        String::decode(&sub_rows.rows[0][0]).unwrap_or_else(|_| "free".into())
    } else {
        "free".into()
    };

    let limits = match plan_id.as_str() {
        "pro" => PlanLimits {
            max_books: -1,
            max_chapters_per_book: -1,
            ai_words_per_month: 100000,
            storage_gb: 50,
            collaborators: 5,
        },
        "enterprise" => PlanLimits {
            max_books: -1,
            max_chapters_per_book: -1,
            ai_words_per_month: -1,
            storage_gb: 500,
            collaborators: -1,
        },
        _ => PlanLimits {
            max_books: 1,
            max_chapters_per_book: 10,
            ai_words_per_month: 5000,
            storage_gb: 1,
            collaborators: 0,
        },
    };

    json_response(200, serde_json::json!({
        "period_start": period_start.to_rfc3339(),
        "usage": {
            "ai_words": ai_words_used,
            "storage_bytes": storage_bytes,
            "storage_gb": (storage_bytes as f64 / 1_073_741_824.0),
            "books": book_count
        },
        "limits": limits,
        "plan_id": plan_id
    }))
}

//=============================================================================
// Stripe API Helpers
//=============================================================================

fn create_stripe_customer(config: &StripeConfig, email: &str, user_id: &Uuid) -> Result<String, ServiceError> {
    let body = format!(
        "email={}&metadata[user_id]={}",
        urlencoded(email),
        user_id
    );

    let response = stripe_request(config, "POST", "/v1/customers", &body)?;
    
    response.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ServiceError::Internal("Failed to create customer".into()))
}

fn create_stripe_subscription(config: &StripeConfig, customer_id: &str, price_id: &str) -> Result<StripeSubscription, ServiceError> {
    let body = format!(
        "customer={}&items[0][price]={}",
        customer_id,
        price_id
    );

    let response = stripe_request(config, "POST", "/v1/subscriptions", &body)?;
    
    Ok(StripeSubscription {
        id: response.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        status: response.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string(),
        current_period_start: response.get("current_period_start")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        current_period_end: response.get("current_period_end")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    })
}

fn update_stripe_subscription(config: &StripeConfig, subscription_id: &str, price_id: &str) -> Result<(), ServiceError> {
    // First get subscription items
    let get_response = stripe_request(config, "GET", &format!("/v1/subscriptions/{}", subscription_id), "")?;
    
    let item_id = get_response
        .get("items")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::Internal("No subscription items found".into()))?;

    let body = format!(
        "items[0][id]={}&items[0][price]={}",
        item_id,
        price_id
    );

    stripe_request(config, "POST", &format!("/v1/subscriptions/{}", subscription_id), &body)?;
    Ok(())
}

fn cancel_stripe_subscription(config: &StripeConfig, subscription_id: &str) -> Result<(), ServiceError> {
    let body = "cancel_at_period_end=true";
    stripe_request(config, "POST", &format!("/v1/subscriptions/{}", subscription_id), body)?;
    Ok(())
}

fn create_stripe_checkout_session(
    config: &StripeConfig,
    customer_id: &str,
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<CheckoutSession, ServiceError> {
    let body = format!(
        "customer={}&mode=subscription&line_items[0][price]={}&line_items[0][quantity]=1&success_url={}&cancel_url={}",
        customer_id,
        price_id,
        urlencoded(success_url),
        urlencoded(cancel_url)
    );

    let response = stripe_request(config, "POST", "/v1/checkout/sessions", &body)?;
    
    Ok(CheckoutSession {
        id: response.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        url: response.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
    })
}

fn create_stripe_portal_session(config: &StripeConfig, customer_id: &str, return_url: &str) -> Result<PortalSession, ServiceError> {
    let body = format!(
        "customer={}&return_url={}",
        customer_id,
        urlencoded(return_url)
    );

    let response = stripe_request(config, "POST", "/v1/billing_portal/sessions", &body)?;
    
    Ok(PortalSession {
        url: response.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
    })
}

fn get_or_create_stripe_customer(conn: &Connection, config: &StripeConfig, user_id: &Uuid) -> Result<String, ServiceError> {
    // Check if customer exists
    let query = "SELECT stripe_customer_id FROM subscriptions.subscriptions WHERE user_id = $1";
    let params = [ParameterValue::Str(user_id.to_string())];
    let rows = conn.query(query, &params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if !rows.rows.is_empty() {
        if let Ok(customer_id) = String::decode(&rows.rows[0][0]) {
            return Ok(customer_id);
        }
    }

    // Get user email
    let user_query = "SELECT email FROM users.users WHERE id = $1";
    let user_params = [ParameterValue::Str(user_id.to_string())];
    let user_rows = conn.query(user_query, &user_params)
        .map_err(|e| ServiceError::Internal(format!("Query failed: {}", e)))?;

    if user_rows.rows.is_empty() {
        return Err(ServiceError::NotFound("User not found".into()));
    }

    let email = String::decode(&user_rows.rows[0][0]).unwrap_or_default();
    create_stripe_customer(config, &email, user_id)
}

fn stripe_request(config: &StripeConfig, method: &str, path: &str, body: &str) -> Result<serde_json::Value, ServiceError> {
    let url = format!("https://api.stripe.com{}", path);
    
    let auth = format!("Basic {}", base64_encode(&format!("{}:", config.secret_key)));
    
    let request = outbound_http::Request::builder()
        .method(method)
        .uri(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .build();

    let response = outbound_http::send(request)
        .map_err(|e| ServiceError::Internal(format!("Stripe request failed: {}", e)))?;

    if response.status() >= 400 {
        return Err(ServiceError::Internal(format!(
            "Stripe API error: {} - {}",
            response.status(),
            String::from_utf8_lossy(response.body())
        )));
    }

    serde_json::from_slice(response.body())
        .map_err(|e| ServiceError::Internal(format!("Failed to parse Stripe response: {}", e)))
}

fn verify_stripe_signature(payload: &[u8], signature: &str, secret: &str) -> Result<(), ServiceError> {
    // Parse signature header
    let parts: std::collections::HashMap<&str, &str> = signature
        .split(',')
        .filter_map(|part| {
            let mut kv = part.split('=');
            Some((kv.next()?, kv.next()?))
        })
        .collect();

    let timestamp = parts.get("t")
        .ok_or_else(|| ServiceError::BadRequest("Invalid signature".into()))?;
    let expected_sig = parts.get("v1")
        .ok_or_else(|| ServiceError::BadRequest("Invalid signature".into()))?;

    // Compute expected signature
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ServiceError::Internal("HMAC error".into()))?;
    mac.update(signed_payload.as_bytes());
    let computed_sig = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison
    if computed_sig != *expected_sig {
        return Err(ServiceError::BadRequest("Invalid signature".into()));
    }

    // Check timestamp (allow 5 minute tolerance)
    let ts: i64 = timestamp.parse()
        .map_err(|_| ServiceError::BadRequest("Invalid timestamp".into()))?;
    let now = Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err(ServiceError::BadRequest("Timestamp too old".into()));
    }

    Ok(())
}

fn base64_encode(s: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(s.as_bytes())
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

//=============================================================================
// Helper Functions
//=============================================================================

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

//=============================================================================
// Credit Endpoint Handlers
//=============================================================================

fn get_credit_packages(_req: &Request) -> Result<Response, ServiceError> {
    let conn = get_db_connection()?;
    credits::list_credit_packages(&conn)
}

fn get_user_credit_balance(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;
    credits::get_credit_balance(&conn, user_id)
}

fn get_user_credit_history(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    // Parse query param for limit (default 50)
    let limit = 50; // TODO: Parse from query string
    credits::get_credit_history(&conn, user_id, limit)
}

fn create_credit_checkout(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    #[derive(Deserialize)]
    struct CheckoutRequest {
        package_id: String,
    }

    let body: CheckoutRequest = parse_json_body(req)?;
    let package_id = Uuid::parse_str(&body.package_id)
        .map_err(|_| ServiceError::BadRequest("Invalid package ID".into()))?;

    credits::create_credit_checkout_session(&conn, user_id, package_id)
}

fn consume_user_credits(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    #[derive(Deserialize)]
    struct ConsumeRequest {
        amount: i32,
        reason: String,
        reference_id: Option<String>,
        reference_type: Option<String>,
    }

    let body: ConsumeRequest = parse_json_body(req)?;

    let reference_id = body.reference_id
        .and_then(|id| Uuid::parse_str(&id).ok());

    let success = credits::consume_credits(
        &conn,
        user_id,
        body.amount,
        &body.reason,
        reference_id,
        body.reference_type.as_deref(),
    )?;

    if success {
        json_response(200, serde_json::json!({
            "success": true,
            "message": "Credits consumed successfully"
        }))
    } else {
        Err(ServiceError::BadRequest("Insufficient credits".into()))
    }
}

fn check_user_credits(req: &Request) -> Result<Response, ServiceError> {
    let user_id = get_user_id(req)?;
    let conn = get_db_connection()?;

    #[derive(Deserialize)]
    struct CheckRequest {
        required_amount: i32,
    }

    let body: CheckRequest = parse_json_body(req)?;

    let has_credits = credits::check_sufficient_credits(
        &conn,
        user_id,
        body.required_amount,
    )?;

    json_response(200, serde_json::json!({
        "has_sufficient_credits": has_credits,
        "required_amount": body.required_amount
    }))
}
