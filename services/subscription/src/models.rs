//! Data models for the Subscription Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;

//=============================================================================
// Configuration
//=============================================================================

#[derive(Debug, Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub price_id_free: String,
    pub price_id_pro: String,
    pub price_id_enterprise: String,
}

//=============================================================================
// Plan Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price_monthly: i64,  // In cents
    pub price_yearly: i64,
    pub features: Vec<String>,
    pub limits: PlanLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    pub max_books: i32,           // -1 for unlimited
    pub max_chapters_per_book: i32,
    pub ai_words_per_month: i64,
    pub storage_gb: i32,
    pub collaborators: i32,
}

//=============================================================================
// Subscription Models
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_subscription_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    pub cancel_at_period_end: bool,
    pub created_at: String,
    pub updated_at: String,
}

//=============================================================================
// Request Models
//=============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan_id: String,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub plan_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub plan_id: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalRequest {
    pub return_url: String,
}

//=============================================================================
// Stripe Response Models
//=============================================================================

#[derive(Debug, Clone)]
pub struct StripeSubscription {
    pub id: String,
    pub status: String,
    pub current_period_start: String,
    pub current_period_end: String,
}

#[derive(Debug, Clone)]
pub struct CheckoutSession {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct PortalSession {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

//=============================================================================
// Subscription Status
//=============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Cancelled,
    Trialing,
    Unpaid,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionStatus::Active => write!(f, "active"),
            SubscriptionStatus::PastDue => write!(f, "past_due"),
            SubscriptionStatus::Cancelled => write!(f, "cancelled"),
            SubscriptionStatus::Trialing => write!(f, "trialing"),
            SubscriptionStatus::Unpaid => write!(f, "unpaid"),
        }
    }
}

