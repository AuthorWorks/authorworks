# Technical Specification: 2D - Subscription Service

## Overview

The Subscription Service manages all payment-related functionality in the AuthorWorks platform, including subscription plans, user subscriptions, payment processing, and billing. This service enables monetization through creator subscriptions, tiered access models, and handling financial transactions.

## Objectives

- Manage subscription plans and tiers for creators
- Process payments securely through integration with payment providers
- Handle subscription lifecycle (creation, renewal, cancellation)
- Provide billing and invoicing capabilities
- Enable revenue sharing between platform and creators
- Track payment history and financial reporting
- Support multiple currencies and payment methods

## Requirements

### 1. Core Subscription Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub name: String,
    pub description: String,
    pub features: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTier {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub name: String,
    pub price_amount: i64,
    pub currency: String,
    pub billing_interval: BillingInterval,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillingInterval {
    Monthly,
    Quarterly,
    Semiannual,
    Annual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub subscriber_id: Uuid,
    pub plan_id: Uuid,
    pub tier_id: Uuid,
    pub status: SubscriptionStatus,
    pub payment_provider: PaymentProvider,
    pub provider_subscription_id: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at_period_end: bool,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub canceled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Trialing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentProvider {
    Stripe,
    PayPal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: InvoiceStatus,
    pub provider_invoice_id: String,
    pub invoice_pdf_url: Option<String>,
    pub invoice_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: PaymentProvider,
    pub provider_payment_method_id: String,
    pub card_brand: Option<String>,
    pub card_last_four: Option<String>,
    pub card_expiry_month: Option<u8>,
    pub card_expiry_year: Option<u16>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueShare {
    pub creator_id: Uuid,
    pub invoice_id: Uuid,
    pub platform_amount: i64,
    pub creator_amount: i64,
    pub currency: String,
    pub status: RevenueShareStatus,
    pub payout_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevenueShareStatus {
    Pending,
    Processed,
    Failed,
}
```

### 2. Database Schema

```sql
CREATE TABLE subscription_plans (
    id UUID PRIMARY KEY,
    creator_id UUID NOT NULL REFERENCES users(id),
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    features JSONB NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE price_tiers (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL REFERENCES subscription_plans(id) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    price_amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    billing_interval VARCHAR(20) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE subscriptions (
    id UUID PRIMARY KEY,
    subscriber_id UUID NOT NULL REFERENCES users(id),
    plan_id UUID NOT NULL REFERENCES subscription_plans(id),
    tier_id UUID NOT NULL REFERENCES price_tiers(id),
    status VARCHAR(20) NOT NULL,
    payment_provider VARCHAR(20) NOT NULL,
    provider_subscription_id VARCHAR(255) NOT NULL,
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    cancellation_reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    canceled_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE invoices (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES subscriptions(id),
    amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(20) NOT NULL,
    provider_invoice_id VARCHAR(255) NOT NULL,
    invoice_pdf_url TEXT,
    invoice_date TIMESTAMP WITH TIME ZONE NOT NULL,
    due_date TIMESTAMP WITH TIME ZONE NOT NULL,
    paid_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE payment_methods (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    provider VARCHAR(20) NOT NULL,
    provider_payment_method_id VARCHAR(255) NOT NULL,
    card_brand VARCHAR(50),
    card_last_four VARCHAR(4),
    card_expiry_month SMALLINT,
    card_expiry_year SMALLINT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(user_id, provider, provider_payment_method_id)
);

CREATE TABLE revenue_shares (
    id UUID PRIMARY KEY,
    creator_id UUID NOT NULL REFERENCES users(id),
    invoice_id UUID NOT NULL REFERENCES invoices(id),
    platform_amount BIGINT NOT NULL,
    creator_amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(20) NOT NULL,
    payout_date TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Indexes
CREATE INDEX idx_subscriptions_subscriber ON subscriptions(subscriber_id);
CREATE INDEX idx_subscriptions_plan ON subscriptions(plan_id);
CREATE INDEX idx_price_tiers_plan ON price_tiers(plan_id);
CREATE INDEX idx_subscription_plans_creator ON subscription_plans(creator_id);
CREATE INDEX idx_invoices_subscription ON invoices(subscription_id);
CREATE INDEX idx_payment_methods_user ON payment_methods(user_id);
CREATE INDEX idx_revenue_shares_creator ON revenue_shares(creator_id);
```

### 3. API Endpoints

```
# Creator Plan Management
POST   /v1/subscription-plans                - Create a new subscription plan
GET    /v1/subscription-plans                - List subscription plans (admin)
GET    /v1/subscription-plans/mine           - List creator's subscription plans
GET    /v1/subscription-plans/{id}           - Get subscription plan details
PUT    /v1/subscription-plans/{id}           - Update subscription plan
DELETE /v1/subscription-plans/{id}           - Delete subscription plan
GET    /v1/subscription-plans/creator/{id}   - Get creator's public plans

# Price Tier Management
POST   /v1/subscription-plans/{plan_id}/tiers       - Create a new price tier
GET    /v1/subscription-plans/{plan_id}/tiers       - List price tiers for a plan
GET    /v1/subscription-plans/{plan_id}/tiers/{id}  - Get price tier details
PUT    /v1/subscription-plans/{plan_id}/tiers/{id}  - Update price tier
DELETE /v1/subscription-plans/{plan_id}/tiers/{id}  - Delete price tier

# Subscription Management
POST   /v1/subscriptions                     - Create a subscription
GET    /v1/subscriptions/mine                - Get user's current subscriptions
GET    /v1/subscriptions/{id}                - Get subscription details
PUT    /v1/subscriptions/{id}/cancel         - Cancel subscription
PUT    /v1/subscriptions/{id}/reactivate     - Reactivate canceled subscription
GET    /v1/subscriptions/creator/{id}        - List subscriptions to a creator (for creator)

# Payment Methods
POST   /v1/payment-methods                   - Add payment method
GET    /v1/payment-methods                   - List user's payment methods
DELETE /v1/payment-methods/{id}              - Remove payment method
PUT    /v1/payment-methods/{id}/default      - Set as default payment method

# Invoice Management
GET    /v1/invoices                          - List user's invoices
GET    /v1/invoices/{id}                     - Get invoice details
GET    /v1/invoices/{id}/pdf                 - Get invoice PDF

# Webhooks
POST   /v1/webhooks/stripe                   - Stripe webhook endpoint
POST   /v1/webhooks/paypal                   - PayPal webhook endpoint

# Analytics
GET    /v1/analytics/revenue                 - Get revenue analytics (for creators)
GET    /v1/analytics/subscriptions           - Get subscription analytics
```

### 4. Payment Integration

The Subscription Service will integrate with payment providers (primarily Stripe) to handle payment processing:

```rust
pub struct PaymentService {
    stripe_client: StripeClient,
    payment_method_repository: Arc<dyn PaymentMethodRepository>,
    subscription_repository: Arc<dyn SubscriptionRepository>,
    plan_repository: Arc<dyn SubscriptionPlanRepository>,
    tier_repository: Arc<dyn PriceTierRepository>,
}

impl PaymentService {
    pub async fn create_customer(
        &self,
        user_id: &Uuid,
        email: &str,
        name: &str,
    ) -> Result<String, Error> {
        let params = CreateCustomer {
            email: Some(email),
            name: Some(name),
            metadata: Some(hashmap! {
                "user_id".to_string() => user_id.to_string(),
            }),
            ..Default::default()
        };
        
        let customer = self.stripe_client.create_customer(params).await?;
        Ok(customer.id)
    }
    
    pub async fn add_payment_method(
        &self,
        user_id: &Uuid,
        payment_method_id: &str,
    ) -> Result<PaymentMethod, Error> {
        let customer = self.get_or_create_customer(user_id).await?;
        
        // Attach payment method to customer
        let params = AttachPaymentMethod {
            customer: &customer,
        };
        
        self.stripe_client.attach_payment_method(payment_method_id, params).await?;
        
        // Retrieve payment method details
        let stripe_payment_method = self.stripe_client.retrieve_payment_method(payment_method_id).await?;
        
        // Set as default if this is the first payment method
        let is_default = !self.payment_method_repository.has_payment_methods(user_id).await?;
        
        if is_default {
            let params = UpdateCustomer {
                invoice_settings: Some(InvoiceSettingsParams {
                    default_payment_method: Some(payment_method_id),
                    ..Default::default()
                }),
                ..Default::default()
            };
            
            self.stripe_client.update_customer(&customer, params).await?;
        }
        
        // Extract card details
        let (card_brand, card_last_four, card_expiry_month, card_expiry_year) = 
            if let Some(card) = &stripe_payment_method.card {
                (
                    Some(card.brand.to_string()),
                    Some(card.last4.clone()),
                    Some(card.exp_month),
                    Some(card.exp_year),
                )
            } else {
                (None, None, None, None)
            };
        
        // Create payment method in database
        let payment_method = PaymentMethod {
            id: Uuid::new_v4(),
            user_id: *user_id,
            provider: PaymentProvider::Stripe,
            provider_payment_method_id: payment_method_id.to_string(),
            card_brand,
            card_last_four,
            card_expiry_month,
            card_expiry_year,
            is_default,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let created = self.payment_method_repository.create(payment_method).await?;
        Ok(created)
    }
    
    pub async fn create_subscription(
        &self,
        user_id: &Uuid,
        tier_id: &Uuid,
        payment_method_id: Option<&Uuid>,
    ) -> Result<Subscription, Error> {
        // Get tier and plan details
        let tier = self.tier_repository.find_by_id(tier_id).await?;
        let plan = self.plan_repository.find_by_id(&tier.plan_id).await?;
        
        // Get or create customer
        let customer_id = self.get_or_create_customer(user_id).await?;
        
        // Get payment method
        let provider_payment_method_id = if let Some(pm_id) = payment_method_id {
            let payment_method = self.payment_method_repository.find_by_id(pm_id).await?;
            payment_method.provider_payment_method_id
        } else {
            // Use default payment method
            let payment_method = self.payment_method_repository.find_default_by_user_id(user_id).await?
                .ok_or(Error::NoDefaultPaymentMethod)?;
            payment_method.provider_payment_method_id
        };
        
        // Create subscription in payment provider
        let params = CreateSubscription {
            customer: &customer_id,
            items: vec![
                CreateSubscriptionItems {
                    price: &self.get_or_create_price(&tier).await?,
                    ..Default::default()
                }
            ],
            default_payment_method: Some(&provider_payment_method_id),
            metadata: Some(hashmap! {
                "user_id".to_string() => user_id.to_string(),
                "plan_id".to_string() => plan.id.to_string(),
                "tier_id".to_string() => tier.id.to_string(),
            }),
            ..Default::default()
        };
        
        let stripe_subscription = self.stripe_client.create_subscription(params).await?;
        
        // Create subscription in database
        let now = Utc::now();
        let subscription = Subscription {
            id: Uuid::new_v4(),
            subscriber_id: *user_id,
            plan_id: plan.id,
            tier_id: *tier_id,
            status: map_subscription_status(&stripe_subscription.status),
            payment_provider: PaymentProvider::Stripe,
            provider_subscription_id: stripe_subscription.id,
            current_period_start: DateTime::from_timestamp(stripe_subscription.current_period_start, 0)
                .unwrap_or(now),
            current_period_end: DateTime::from_timestamp(stripe_subscription.current_period_end, 0)
                .unwrap_or(now + chrono::Duration::days(30)),
            cancel_at_period_end: stripe_subscription.cancel_at_period_end,
            cancellation_reason: None,
            created_at: now,
            updated_at: now,
            canceled_at: None,
        };
        
        let created = self.subscription_repository.create(subscription).await?;
        Ok(created)
    }
    
    pub async fn cancel_subscription(
        &self,
        subscription_id: &Uuid,
        cancel_immediately: bool,
        reason: Option<String>,
    ) -> Result<Subscription, Error> {
        let subscription = self.subscription_repository.find_by_id(subscription_id).await?;
        
        if cancel_immediately {
            // Cancel immediately
            let params = CancelSubscription {
                ..Default::default()
            };
            
            self.stripe_client.cancel_subscription(
                &subscription.provider_subscription_id,
                params,
            ).await?;
            
            // Update subscription in database
            let now = Utc::now();
            let updated = self.subscription_repository.update_status(
                subscription_id,
                SubscriptionStatus::Canceled,
                Some(reason),
                Some(now),
            ).await?;
            
            Ok(updated)
        } else {
            // Cancel at period end
            let params = UpdateSubscription {
                cancel_at_period_end: Some(true),
                ..Default::default()
            };
            
            self.stripe_client.update_subscription(
                &subscription.provider_subscription_id,
                params,
            ).await?;
            
            // Update subscription in database
            let updated = self.subscription_repository.update_cancel_at_period_end(
                subscription_id,
                true,
                reason,
            ).await?;
            
            Ok(updated)
        }
    }
    
    // Helper method to get or create Stripe customer
    async fn get_or_create_customer(&self, user_id: &Uuid) -> Result<String, Error> {
        // Implementation omitted for brevity
        // This would check for existing customer ID in user metadata
        // or create a new customer if none exists
        todo!()
    }
    
    // Helper method to get or create Stripe price for a tier
    async fn get_or_create_price(&self, tier: &PriceTier) -> Result<String, Error> {
        // Implementation omitted for brevity
        // This would check for existing price ID in tier metadata
        // or create a new price if none exists
        todo!()
    }
}

// Helper function to map Stripe subscription status to our enum
fn map_subscription_status(status: &str) -> SubscriptionStatus {
    match status {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        "trialing" => SubscriptionStatus::Trialing,
        _ => SubscriptionStatus::Active,
    }
}
```

### 5. Revenue Sharing

```rust
pub struct RevenueService {
    revenue_share_repository: Arc<dyn RevenueShareRepository>,
    invoice_repository: Arc<dyn InvoiceRepository>,
    subscription_repository: Arc<dyn SubscriptionRepository>,
    plan_repository: Arc<dyn SubscriptionPlanRepository>,
}

impl RevenueService {
    pub async fn process_payment(
        &self,
        invoice_id: &Uuid,
    ) -> Result<RevenueShare, Error> {
        let invoice = self.invoice_repository.find_by_id(invoice_id).await?;
        
        if invoice.status != InvoiceStatus::Paid {
            return Err(Error::InvoiceNotPaid);
        }
        
        // Get subscription and plan details
        let subscription = self.subscription_repository.find_by_id(&invoice.subscription_id).await?;
        let plan = self.plan_repository.find_by_id(&subscription.plan_id).await?;
        
        // Calculate revenue share
        // Platform takes 10% of subscription price
        let platform_percentage = 0.10;
        let platform_amount = (invoice.amount as f64 * platform_percentage).round() as i64;
        let creator_amount = invoice.amount - platform_amount;
        
        // Create revenue share record
        let revenue_share = RevenueShare {
            id: Uuid::new_v4(),
            creator_id: plan.creator_id,
            invoice_id: *invoice_id,
            platform_amount,
            creator_amount,
            currency: invoice.currency.clone(),
            status: RevenueShareStatus::Pending,
            payout_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let created = self.revenue_share_repository.create(revenue_share).await?;
        Ok(created)
    }
    
    pub async fn process_creator_payouts(&self) -> Result<Vec<RevenueShare>, Error> {
        // Get all pending revenue shares
        let pending_shares = self.revenue_share_repository
            .find_by_status(RevenueShareStatus::Pending)
            .await?;
        
        let mut processed_shares = Vec::new();
        
        // Group by creator and currency for batch processing
        let mut creator_batches: HashMap<(Uuid, String), Vec<RevenueShare>> = HashMap::new();
        
        for share in pending_shares {
            let key = (share.creator_id, share.currency.clone());
            creator_batches.entry(key).or_default().push(share);
        }
        
        // Process each batch
        for ((creator_id, currency), shares) in creator_batches {
            // Calculate total amount
            let total_amount: i64 = shares.iter().map(|s| s.creator_amount).sum();
            
            // Process payout to creator (integration with payment provider)
            if let Ok(_) = self.process_creator_payout(&creator_id, total_amount, &currency).await {
                // Update status for all shares in this batch
                let now = Utc::now();
                for share in &shares {
                    let updated = self.revenue_share_repository
                        .update_status(&share.id, RevenueShareStatus::Processed, Some(now))
                        .await?;
                    processed_shares.push(updated);
                }
            }
        }
        
        Ok(processed_shares)
    }
    
    async fn process_creator_payout(
        &self,
        creator_id: &Uuid,
        amount: i64,
        currency: &str,
    ) -> Result<(), Error> {
        // Implementation omitted for brevity
        // This would integrate with a payment provider to transfer funds to the creator
        todo!()
    }
    
    pub async fn get_creator_revenue(
        &self,
        creator_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<CreatorRevenueReport, Error> {
        // Get all processed revenue shares for the creator in date range
        let shares = self.revenue_share_repository
            .find_by_creator_and_date_range(creator_id, start_date, end_date)
            .await?;
        
        // Group by currency
        let mut by_currency: HashMap<String, i64> = HashMap::new();
        let mut total_count = 0;
        
        for share in shares {
            let entry = by_currency.entry(share.currency.clone()).or_default();
            *entry += share.creator_amount;
            total_count += 1;
        }
        
        // Build report
        let report = CreatorRevenueReport {
            creator_id: *creator_id,
            start_date,
            end_date,
            total_count,
            by_currency,
        };
        
        Ok(report)
    }
}

#[derive(Debug, Serialize)]
pub struct CreatorRevenueReport {
    pub creator_id: Uuid,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_count: usize,
    pub by_currency: HashMap<String, i64>,
}
```

### 6. Webhook Handling

```rust
pub struct WebhookService {
    stripe_client: StripeClient,
    subscription_repository: Arc<dyn SubscriptionRepository>,
    invoice_repository: Arc<dyn InvoiceRepository>,
    revenue_service: Arc<RevenueService>,
}

impl WebhookService {
    pub async fn handle_stripe_webhook(
        &self,
        signature: &str,
        body: &[u8],
    ) -> Result<(), Error> {
        // Verify webhook signature
        let event = self.stripe_client.construct_event(body, signature)?;
        
        match event.type_.as_str() {
            "invoice.paid" => {
                self.handle_invoice_paid(&event).await?;
            },
            "invoice.payment_failed" => {
                self.handle_invoice_payment_failed(&event).await?;
            },
            "customer.subscription.updated" => {
                self.handle_subscription_updated(&event).await?;
            },
            "customer.subscription.deleted" => {
                self.handle_subscription_deleted(&event).await?;
            },
            _ => {
                // Ignore other event types
            }
        }
        
        Ok(())
    }
    
    async fn handle_invoice_paid(&self, event: &Event) -> Result<(), Error> {
        let invoice: Invoice = event.data.object.clone().into_object()?;
        
        // Find subscription
        let subscription_id = invoice.subscription.ok_or(Error::MissingSubscription)?;
        let subscription = self.subscription_repository
            .find_by_provider_id(&subscription_id)
            .await?;
        
        // Create invoice record
        let invoice_record = self.create_invoice_record(&subscription, &invoice, InvoiceStatus::Paid).await?;
        
        // Process revenue sharing
        self.revenue_service.process_payment(&invoice_record.id).await?;
        
        Ok(())
    }
    
    async fn handle_invoice_payment_failed(&self, event: &Event) -> Result<(), Error> {
        let invoice: Invoice = event.data.object.clone().into_object()?;
        
        // Find subscription
        let subscription_id = invoice.subscription.ok_or(Error::MissingSubscription)?;
        let subscription = self.subscription_repository
            .find_by_provider_id(&subscription_id)
            .await?;
        
        // Create invoice record
        self.create_invoice_record(&subscription, &invoice, InvoiceStatus::Uncollectible).await?;
        
        // Update subscription status if needed
        if invoice.attempted {
            self.subscription_repository
                .update_status(&subscription.id, SubscriptionStatus::PastDue, None, None)
                .await?;
        }
        
        Ok(())
    }
    
    async fn handle_subscription_updated(&self, event: &Event) -> Result<(), Error> {
        let stripe_subscription: Subscription = event.data.object.clone().into_object()?;
        
        // Find subscription
        let subscription = self.subscription_repository
            .find_by_provider_id(&stripe_subscription.id)
            .await?;
        
        // Update subscription details
        let status = map_subscription_status(&stripe_subscription.status);
        let current_period_start = DateTime::from_timestamp(stripe_subscription.current_period_start, 0)
            .unwrap_or(Utc::now());
        let current_period_end = DateTime::from_timestamp(stripe_subscription.current_period_end, 0)
            .unwrap_or(Utc::now() + chrono::Duration::days(30));
        
        self.subscription_repository
            .update_subscription_details(
                &subscription.id,
                status,
                current_period_start,
                current_period_end,
                stripe_subscription.cancel_at_period_end,
            )
            .await?;
        
        Ok(())
    }
    
    async fn handle_subscription_deleted(&self, event: &Event) -> Result<(), Error> {
        let stripe_subscription: Subscription = event.data.object.clone().into_object()?;
        
        // Find subscription
        let subscription = self.subscription_repository
            .find_by_provider_id(&stripe_subscription.id)
            .await?;
        
        // Update subscription status
        self.subscription_repository
            .update_status(
                &subscription.id,
                SubscriptionStatus::Canceled,
                None,
                Some(Utc::now()),
            )
            .await?;
        
        Ok(())
    }
    
    async fn create_invoice_record(
        &self,
        subscription: &Subscription,
        stripe_invoice: &Invoice,
        status: InvoiceStatus,
    ) -> Result<models::Invoice, Error> {
        let invoice = models::Invoice {
            id: Uuid::new_v4(),
            subscription_id: subscription.id,
            amount: stripe_invoice.amount_paid,
            currency: stripe_invoice.currency.clone(),
            status,
            provider_invoice_id: stripe_invoice.id.clone(),
            invoice_pdf_url: stripe_invoice.invoice_pdf.clone(),
            invoice_date: DateTime::from_timestamp(stripe_invoice.created, 0).unwrap_or(Utc::now()),
            due_date: DateTime::from_timestamp(stripe_invoice.due_date.unwrap_or(0), 0)
                .unwrap_or(Utc::now()),
            paid_at: if status == InvoiceStatus::Paid {
                Some(Utc::now())
            } else {
                None
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let created = self.invoice_repository.create(invoice).await?;
        Ok(created)
    }
}
```

## Implementation Steps

1. Set up project structure and database schema
2. Implement basic CRUD operations for subscription plans and tiers
3. Create Stripe integration for payment processing
4. Implement subscription management functionality
5. Set up webhook handling for payment events
6. Create revenue sharing and payout system
7. Implement analytics and reporting features
8. Add comprehensive tests for payment flows
9. Set up monitoring and alerting for payment failures
10. Document API endpoints and integration examples

## Technical Decisions

### Why Stripe as Primary Payment Provider?

Stripe was chosen as the primary payment provider because:
- Comprehensive API for subscription management
- Support for multiple payment methods and currencies
- Strong security features and compliance certifications
- Webhook system for real-time event notifications
- Robust documentation and SDK support
- Global coverage for international payments

### Database Design Considerations

The database schema separates subscription plans from price tiers to:
- Allow creators to offer the same plan at different price points
- Support different billing intervals for the same plan
- Enable A/B testing of pricing strategies
- Simplify plan updates without affecting existing subscriptions

## Success Criteria

The Subscription Service will be considered successfully implemented when:

1. Creators can create and manage subscription plans with multiple price tiers
2. Users can subscribe to creator plans with different payment methods
3. Subscriptions are correctly managed throughout their lifecycle
4. Revenue sharing is properly calculated and tracked
5. Payment provider webhooks are processed correctly
6. Financial reporting is accurate and comprehensive
7. System handles payment failures gracefully with appropriate retries
8. All monetary transactions are properly recorded for accounting purposes 