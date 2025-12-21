# AuthorWorks Production Setup Guide

**Version:** 1.0
**Date:** December 19, 2025
**Status:** Production Ready

---

## Overview

This guide covers the complete production setup for AuthorWorks, including:
- User authentication via Logto (OAuth2/OIDC)
- Payment processing via Stripe (subscriptions + one-time credit purchases)
- Credit system for AI content generation
- Database setup (PostgreSQL with full schema)
- K3s/Kubernetes deployment

---

## Prerequisites

- Kubernetes cluster (K3s recommended) with kubectl access
- PostgreSQL 15+ database
- Stripe account with API access
- Logto instance (self-hosted or cloud)
- Domain with SSL certificates (Let's Encrypt via Traefik)

---

## Part 1: Database Setup

### Step 1: Deploy PostgreSQL

**Local Development (Docker):**
```bash
docker run -d \
  --name authorworks-postgres \
  --network authorworks \
  -e POSTGRES_DB=authorworks \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=your_secure_password \
  -p 5432:5432 \
  postgres:15-alpine
```

**K3s Production:**
- Use existing `neon-postgres-leopaska` service at `neon-postgres-leopaska:5432`
- Database: `authorworks`
- User: `postgres`

### Step 2: Run Database Migrations

1. Apply base schema:
```bash
psql -h localhost -U postgres -d authorworks < scripts/schema.sql
```

2. Apply credit system migration:
```bash
psql -h localhost -U postgres -d authorworks < scripts/migrations/001_add_credit_system.sql
```

3. Verify migrations:
```bash
psql -h localhost -U postgres -d authorworks -c "\dt subscriptions.*"
```

Expected tables:
- `subscriptions.subscriptions`
- `subscriptions.invoices`
- `subscriptions.ai_usage`
- `subscriptions.credit_packages` (NEW)
- `subscriptions.credits` (NEW)
- `subscriptions.credit_transactions` (NEW)
- `subscriptions.credit_orders` (NEW)

### Step 3: Verify Seed Data

Check that default credit packages were created:
```sql
SELECT id, name, credit_amount, price_cents
FROM subscriptions.credit_packages
ORDER BY sort_order;
```

Expected output:
```
| name             | credit_amount | price_cents |
|------------------|---------------|-------------|
| Starter Pack     | 1,000         | 999         |
| Writer Pack      | 5,000         | 3,999       |
| Author Pack      | 15,000        | 9,999       |
| Publisher Pack   | 50,000        | 29,999      |
```

---

## Part 2: Stripe Configuration

### Step 1: Create Stripe Account

1. Go to [https://dashboard.stripe.com/register](https://dashboard.stripe.com/register)
2. Complete registration and verification
3. Enable Test Mode for initial setup

### Step 2: Create Subscription Products

Navigate to **Products** → **Add Product**

**Product 1: Professional Plan**
- Name: "AuthorWorks Professional"
- Description: "For serious authors"
- Price: $19.99/month (recurring)
- Copy the Price ID: `price_...` → Save as `STRIPE_PRICE_PRO`

**Product 2: Enterprise Plan**
- Name: "AuthorWorks Enterprise"
- Description: "For publishing teams"
- Price: $99.99/month (recurring)
- Copy the Price ID: `price_...` → Save as `STRIPE_PRICE_ENTERPRISE`

### Step 3: Create Credit Package Products

Create one-time payment products for credit purchases:

**Package 1: Starter Pack (1,000 credits)**
- Name: "1,000 AI Credits"
- Description: "Starter pack for book generation"
- Price: $9.99 (one-time)
- Copy Price ID → Save as `STRIPE_CREDIT_STARTER`

**Package 2: Writer Pack (5,000 credits)**
- Name: "5,000 AI Credits"
- Description: "Writer pack for multiple books"
- Price: $39.99 (one-time)
- Copy Price ID → Save as `STRIPE_CREDIT_WRITER`

**Package 3: Author Pack (15,000 credits)**
- Name: "15,000 AI Credits"
- Description: "Author pack for serious writers"
- Price: $99.99 (one-time)
- Copy Price ID → Save as `STRIPE_CREDIT_AUTHOR`

**Package 4: Publisher Pack (50,000 credits)**
- Name: "50,000 AI Credits"
- Description: "Publisher pack for large projects"
- Price: $299.99 (one-time)
- Copy Price ID → Save as `STRIPE_CREDIT_PUBLISHER`

### Step 4: Update Database with Stripe Price IDs

```sql
UPDATE subscriptions.credit_packages
SET stripe_price_id = 'price_...'
WHERE name = 'Starter Pack';

UPDATE subscriptions.credit_packages
SET stripe_price_id = 'price_...'
WHERE name = 'Writer Pack';

-- Repeat for other packages
```

### Step 5: Configure Webhooks

1. Go to **Developers** → **Webhooks**
2. Click **Add endpoint**
3. Endpoint URL: `https://api.your-domain.com/subscription/webhooks/stripe`
4. Select events:
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.paid`
   - `invoice.payment_failed`
   - `checkout.session.completed`
5. Copy **Signing secret** → Save as `STRIPE_WEBHOOK_SECRET`

### Step 6: Get API Keys

1. Go to **Developers** → **API keys**
2. Copy **Secret key** → Save as `STRIPE_SECRET_KEY`
3. Copy **Publishable key** → Save as `STRIPE_PUBLISHABLE_KEY`

---

## Part 3: Logto Authentication Setup

### Step 1: Deploy Logto (if not already deployed)

Your K3s setup already has Logto running at `auth.leopaska.xyz`

Verify deployment:
```bash
kubectl get pods -n authorworks | grep logto
kubectl get ingress -n authorworks | grep logto
```

### Step 2: Configure Application in Logto

1. Access Logto Admin Console: `https://auth-admin.author.works`
2. Navigate to **Applications** → **Create Application**
3. Application type: **Traditional Web**
4. Name: "AuthorWorks"
5. Configure redirect URIs:
   - `https://author.works/api/auth/callback`
   - `https://author.works/auth/callback`
   - `http://localhost:3000/api/auth/callback` (for local dev)

### Step 3: Configure Social Sign-In (Optional)

Navigate to **Sign-in Experience** → **Social connectors**

Add connectors:
- Google
- GitHub
- Apple
- Twitter

### Step 4: Save Credentials

Copy from Application details:
- App ID → `LOGTO_APP_ID`
- App Secret → `LOGTO_APP_SECRET`
- Endpoint → `LOGTO_ENDPOINT` (e.g., `https://auth.leopaska.xyz`)

---

## Part 4: Environment Configuration

### Step 1: Update `.env` File

Edit `/home/l3o/git/production/authorworks/.env`:

```bash
# Database
DATABASE_URL=postgresql://postgres:your_password@neon-postgres-leopaska:5432/authorworks

# Stripe (REQUIRED)
STRIPE_SECRET_KEY=sk_test_xxxxxxxxxxxxx
STRIPE_WEBHOOK_SECRET=whsec_xxxxxxxxxxxxx
STRIPE_PUBLISHABLE_KEY=pk_test_xxxxxxxxxxxxx

# Stripe Price IDs - Subscriptions
STRIPE_PRICE_FREE=price_free
STRIPE_PRICE_PRO=price_1xxxxxxxxxx
STRIPE_PRICE_ENTERPRISE=price_1xxxxxxxxxx

# Stripe Price IDs - Credits
STRIPE_CREDIT_STARTER=price_1xxxxxxxxxx
STRIPE_CREDIT_WRITER=price_1xxxxxxxxxx
STRIPE_CREDIT_AUTHOR=price_1xxxxxxxxxx
STRIPE_CREDIT_PUBLISHER=price_1xxxxxxxxxx

# Logto
LOGTO_ENDPOINT=https://auth.leopaska.xyz
LOGTO_APP_ID=xxxxxxxxxxxxx
LOGTO_APP_SECRET=xxxxxxxxxxxxx

# Service URLs
SUBSCRIPTION_SERVICE_URL=http://subscription-service:3105
CONTENT_SERVICE_URL=http://content-service:3102
USER_SERVICE_URL=http://user-service:3101
```

### Step 2: Create Kubernetes Secrets

```bash
# From project root
cd /home/l3o/git/production/authorworks

# Create secret from .env file
kubectl create secret generic authorworks-env \
  --from-env-file=.env \
  -n authorworks \
  --dry-run=client -o yaml | kubectl apply -f -
```

---

## Part 5: K3s Deployment

### Step 1: Deploy Services

```bash
# Deploy all services
kubectl apply -f k8s/base/

# Or deploy individually
kubectl apply -f k8s/base/user-service.yaml
kubectl apply -f k8s/base/subscription-service.yaml
kubectl apply -f k8s/base/content-service.yaml
kubectl apply -f k8s/base/frontend.yaml
```

### Step 2: Verify Deployments

```bash
# Check pods
kubectl get pods -n authorworks

# Check services
kubectl get svc -n authorworks

# Check ingress
kubectl get ingress -n authorworks
```

Expected pods:
- `user-service-xxx`
- `subscription-service-xxx`
- `content-service-xxx`
- `frontend-xxx`
- `logto-xxx`

### Step 3: Apply Database Migrations

```bash
# Connect to database from inside cluster
kubectl run psql-client --rm -it \
  --image=postgres:15-alpine \
  --command -- psql postgresql://postgres:password@neon-postgres-leopaska:5432/authorworks

# Inside psql, run migrations
\i /migrations/001_add_credit_system.sql
```

Alternative: Copy SQL files to a pod and execute:
```bash
kubectl cp scripts/migrations/001_add_credit_system.sql \
  authorworks/user-service-xxx:/tmp/migration.sql

kubectl exec -it user-service-xxx -n authorworks -- \
  psql $DATABASE_URL -f /tmp/migration.sql
```

---

## Part 6: Testing the Platform

### Step 1: Test Authentication

1. Visit `https://author.works`
2. Click "Sign Up" or "Log In"
3. Should redirect to Logto: `https://auth.leopaska.xyz`
4. Complete authentication
5. Should redirect back with JWT token

### Step 2: Test Credit Purchase Flow

**API Test:**
```bash
# Get credit packages
curl https://api.author.works/subscription/credits/packages

# Check user balance (requires auth token)
curl -H "Authorization: Bearer $TOKEN" \
     -H "X-User-Id: $USER_ID" \
     https://api.author.works/subscription/credits/balance

# Create checkout session
curl -X POST https://api.author.works/subscription/credits/checkout \
     -H "Authorization: Bearer $TOKEN" \
     -H "X-User-Id: $USER_ID" \
     -H "Content-Type: application/json" \
     -d '{"package_id": "uuid-of-package"}'
```

**UI Test:**
1. Log in to platform
2. Navigate to "Buy Credits" page
3. Select a credit package
4. Click "Purchase"
5. Complete Stripe checkout (use test card `4242 4242 4242 4242`)
6. Verify credits are added to account

### Step 3: Test Content Generation with Credits

```bash
# Generate outline (should consume ~50 credits)
curl -X POST https://api.author.works/content/generate/outline \
     -H "Authorization: Bearer $TOKEN" \
     -H "X-User-Id: $USER_ID" \
     -H "Content-Type: application/json" \
     -d '{
       "book_id": "uuid",
       "prompt": "A mystery novel set in Victorian London",
       "genre": "mystery",
       "chapter_count": 10
     }'

# Check credit balance again (should be reduced)
curl -H "Authorization: Bearer $TOKEN" \
     -H "X-User-Id: $USER_ID" \
     https://api.author.works/subscription/credits/balance
```

### Step 4: Test Insufficient Credits

```bash
# Set user balance to 10 credits (admin operation)
psql $DATABASE_URL -c "UPDATE subscriptions.credits SET balance = 10 WHERE user_id = '$USER_ID'"

# Try to generate chapter (requires ~250 credits)
curl -X POST https://api.author.works/content/generate/chapter \
     -H "Authorization: Bearer $TOKEN" \
     -H "X-User-Id: $USER_ID" \
     -H "Content-Type: application/json" \
     -d '{
       "chapter_id": "uuid",
       "outline": "Chapter outline here",
       "target_length": 2500
     }'

# Expected response: 402 Payment Required
# {
#   "error": "Payment required: Insufficient credits. Required: 250, Available: 10",
#   "code": "PAYMENT_REQUIRED"
# }
```

---

## Part 7: Monitoring & Operations

### Check Service Health

```bash
# Health checks
curl https://api.author.works/user/health
curl https://api.author.works/subscription/health
curl https://api.author.works/content/health
```

### Monitor Credit Usage

```sql
-- Top credit consumers
SELECT u.email, c.balance, c.total_consumed, c.total_purchased
FROM subscriptions.credits c
JOIN users.users u ON c.user_id = u.id
ORDER BY c.total_consumed DESC
LIMIT 20;

-- Recent credit transactions
SELECT t.user_id, t.amount, t.transaction_type, t.reason, t.created_at
FROM subscriptions.credit_transactions t
ORDER BY t.created_at DESC
LIMIT 50;

-- Credit package sales
SELECT * FROM subscriptions.v_credit_package_sales;
```

### Monitor Stripe Webhooks

```bash
# View webhook logs
kubectl logs -f subscription-service-xxx -n authorworks | grep webhook

# Test webhook delivery in Stripe Dashboard
# Developers → Webhooks → Select endpoint → Send test webhook
```

---

## Part 8: Going Live (Production)

### Step 1: Switch Stripe to Live Mode

1. Stripe Dashboard → Toggle "Test mode" to OFF
2. Update API keys in `.env` with live keys:
   - `STRIPE_SECRET_KEY=sk_live_...`
   - `STRIPE_WEBHOOK_SECRET=whsec_...` (create new webhook for live mode)
   - `STRIPE_PUBLISHABLE_KEY=pk_live_...`

3. Update Kubernetes secret:
```bash
kubectl create secret generic authorworks-env \
  --from-env-file=.env \
  -n authorworks \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart services to pick up new env
kubectl rollout restart deployment -n authorworks
```

### Step 2: Production Database Backup

```bash
# Setup automated backups
kubectl apply -f k8s/base/postgres-backup-cronjob.yaml

# Manual backup
kubectl exec -it neon-postgres-leopaska-0 -n authorworks -- \
  pg_dump -U postgres authorworks | gzip > backup-$(date +%Y%m%d).sql.gz
```

### Step 3: Enable SSL/TLS

Traefik should automatically handle Let's Encrypt certificates for:
- `https://author.works`
- `https://api.author.works`
- `https://auth.leopaska.xyz`

Verify:
```bash
kubectl get certificate -n authorworks
```

### Step 4: Production Checklist

- [ ] Database migrations applied
- [ ] Stripe live mode enabled
- [ ] Webhook endpoints configured and tested
- [ ] Logto social connectors enabled
- [ ] SSL certificates active
- [ ] Monitoring/alerting configured
- [ ] Backup strategy implemented
- [ ] Rate limiting enabled
- [ ] CORS configured correctly
- [ ] API keys rotated from defaults
- [ ] Security audit completed

---

## Troubleshooting

### Issue: Credits not deducted after generation

**Cause:** Subscription service URL not configured

**Fix:**
```bash
# Verify service can reach subscription service
kubectl exec -it content-service-xxx -n authorworks -- \
  curl http://subscription-service:3105/health

# Check environment variable
kubectl exec -it content-service-xxx -n authorworks -- \
  env | grep SUBSCRIPTION_SERVICE_URL
```

### Issue: Stripe webhook failing

**Cause:** Webhook signature verification failed

**Fix:**
1. Check `STRIPE_WEBHOOK_SECRET` matches Stripe Dashboard
2. Verify webhook endpoint is publicly accessible
3. Check logs: `kubectl logs subscription-service-xxx | grep stripe`

### Issue: Logto authentication not working

**Cause:** Redirect URI mismatch

**Fix:**
1. Verify redirect URIs in Logto Application settings
2. Check `LOGTO_ENDPOINT` is correct
3. Ensure CORS is configured:
```yaml
# In Logto config
allowed_origins:
  - https://author.works
  - http://localhost:3000
```

---

## API Reference

### Credit Endpoints

**GET /subscription/credits/packages**
- Returns list of available credit packages
- Public endpoint (no auth required)

**GET /subscription/credits/balance**
- Returns user's current credit balance
- Requires: Authorization header + X-User-Id

**GET /subscription/credits/history?limit=50**
- Returns user's credit transaction history
- Requires: Authorization header + X-User-Id

**POST /subscription/credits/checkout**
- Creates Stripe checkout session for credit purchase
- Body: `{"package_id": "uuid"}`
- Returns: `{"checkout_url": "https://checkout.stripe.com/..."}`

**POST /subscription/credits/consume**
- Consumes credits (internal use by content service)
- Body: `{"amount": 100, "reason": "book generation", "reference_id": "uuid"}`

**POST /subscription/credits/check**
- Checks if user has sufficient credits
- Body: `{"required_amount": 250}`
- Returns: `{"has_sufficient_credits": true/false}`

### Content Generation Endpoints (with Credit Enforcement)

**POST /content/generate/outline**
- Cost: ~50 credits
- Returns 402 if insufficient credits

**POST /content/generate/chapter**
- Cost: 1 credit per 10 words (e.g., 2500 words = 250 credits)
- Returns 402 if insufficient credits

**POST /content/generate/enhance**
- Cost: 1 credit per 20 words
- Returns 402 if insufficient credits

---

## Cost Estimation

### Credit Costs per Generation Type

- **Outline**: 50 credits flat
- **Chapter** (2500 words): 250 credits
- **Enhancement**: 5 credits per 100 words

### Example Book Costs

**Short Novel (50,000 words, 20 chapters)**
- Outline: 50 credits
- 20 chapters × 250 credits: 5,000 credits
- Enhancements (10% of text): 250 credits
- **Total**: ~5,300 credits (~$53 at $9.99/1000 credits)

**Full Novel (90,000 words, 30 chapters)**
- Outline: 50 credits
- 30 chapters × 300 credits: 9,000 credits
- Enhancements: 450 credits
- **Total**: ~9,500 credits (~$95 at $9.99/1000 credits)

---

## Next Steps

1. **Mobile App**: Deploy the Tauri mobile app (see `/mobile/README.md`)
2. **Analytics**: Add usage analytics dashboard
3. **Notifications**: Set up email notifications for low credit balance
4. **Referral Program**: Implement credit rewards for referrals
5. **Bulk Discounts**: Add enterprise pricing for high-volume users

---

## Support

- **Documentation**: `/docs/*`
- **API Docs**: `https://api.author.works/docs`
- **GitHub Issues**: `https://github.com/your-org/authorworks/issues`
- **Email**: support@author.works

---

**Last Updated**: December 19, 2025
**Version**: 1.0
**Status**: Production Ready ✅
