-- Migration: 001 - Add Credit System
-- Description: Adds credit purchase, tracking, and consumption tables
-- Date: 2025-12-19
-- Author: AuthorWorks Team

--=============================================================================
-- CREDIT SYSTEM TABLES
--=============================================================================

-- Credit packages available for purchase
CREATE TABLE IF NOT EXISTS subscriptions.credit_packages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    credit_amount INTEGER NOT NULL,
    price_cents INTEGER NOT NULL,
    stripe_price_id VARCHAR(255) UNIQUE,
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User credit balances
CREATE TABLE IF NOT EXISTS subscriptions.credits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    balance INTEGER NOT NULL DEFAULT 0,
    total_purchased INTEGER DEFAULT 0,
    total_consumed INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Credit transaction history
CREATE TABLE IF NOT EXISTS subscriptions.credit_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,  -- Positive = purchase/refund, Negative = consumption
    balance_after INTEGER NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,  -- 'purchase', 'consumption', 'refund', 'admin_adjustment'
    reason VARCHAR(255),
    reference_id UUID,  -- book_id, chapter_id, or order_id
    reference_type VARCHAR(50),  -- 'book', 'chapter', 'order', 'generation_job'
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Credit orders (Stripe payment records)
CREATE TABLE IF NOT EXISTS subscriptions.credit_orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    package_id UUID REFERENCES subscriptions.credit_packages(id),
    credit_amount INTEGER NOT NULL,
    price_cents INTEGER NOT NULL,
    stripe_payment_intent_id VARCHAR(255),
    stripe_checkout_session_id VARCHAR(255),
    status VARCHAR(50) DEFAULT 'pending',  -- 'pending', 'completed', 'failed', 'refunded'
    completed_at TIMESTAMPTZ,
    refunded_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

--=============================================================================
-- ADD COST TRACKING TO CONTENT
--=============================================================================

-- Add credit cost tracking to books
ALTER TABLE content.books
ADD COLUMN IF NOT EXISTS credits_used INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS estimated_cost INTEGER DEFAULT 0;

-- Add cost tracking to chapters
ALTER TABLE content.chapters
ADD COLUMN IF NOT EXISTS credits_used INTEGER DEFAULT 0;

-- Add cost tracking to generation jobs
ALTER TABLE content.generation_jobs
ADD COLUMN IF NOT EXISTS credits_cost INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS credits_charged BOOLEAN DEFAULT FALSE;

--=============================================================================
-- INDEXES
--=============================================================================

CREATE INDEX IF NOT EXISTS idx_credits_user ON subscriptions.credits(user_id);
CREATE INDEX IF NOT EXISTS idx_credit_transactions_user ON subscriptions.credit_transactions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_credit_transactions_reference ON subscriptions.credit_transactions(reference_id, reference_type);
CREATE INDEX IF NOT EXISTS idx_credit_orders_user ON subscriptions.credit_orders(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_credit_orders_stripe ON subscriptions.credit_orders(stripe_payment_intent_id);
CREATE INDEX IF NOT EXISTS idx_credit_packages_active ON subscriptions.credit_packages(is_active, sort_order);

--=============================================================================
-- TRIGGERS
--=============================================================================

-- Auto-update updated_at timestamp on credits
CREATE OR REPLACE FUNCTION subscriptions.update_credits_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_credits_timestamp
    BEFORE UPDATE ON subscriptions.credits
    FOR EACH ROW
    EXECUTE FUNCTION subscriptions.update_credits_timestamp();

-- Auto-update updated_at timestamp on credit_packages
CREATE TRIGGER trigger_update_credit_packages_timestamp
    BEFORE UPDATE ON subscriptions.credit_packages
    FOR EACH ROW
    EXECUTE FUNCTION subscriptions.update_credits_timestamp();

-- Auto-update updated_at timestamp on credit_orders
CREATE TRIGGER trigger_update_credit_orders_timestamp
    BEFORE UPDATE ON subscriptions.credit_orders
    FOR EACH ROW
    EXECUTE FUNCTION subscriptions.update_credits_timestamp();

--=============================================================================
-- SEED DATA: DEFAULT CREDIT PACKAGES
--=============================================================================

INSERT INTO subscriptions.credit_packages (name, description, credit_amount, price_cents, sort_order, metadata) VALUES
('Starter Pack', '1,000 AI credits for book generation', 1000, 999, 1, '{"words_equivalent": 10000}'),
('Writer Pack', '5,000 AI credits for multiple books', 5000, 3999, 2, '{"words_equivalent": 50000, "discount_percent": 20}'),
('Author Pack', '15,000 AI credits for serious authors', 15000, 9999, 3, '{"words_equivalent": 150000, "discount_percent": 33}'),
('Publisher Pack', '50,000 AI credits for publishing projects', 50000, 29999, 4, '{"words_equivalent": 500000, "discount_percent": 40}')
ON CONFLICT DO NOTHING;

--=============================================================================
-- FUNCTIONS: CREDIT MANAGEMENT
--=============================================================================

-- Function to get user credit balance
CREATE OR REPLACE FUNCTION subscriptions.get_credit_balance(p_user_id UUID)
RETURNS INTEGER AS $$
DECLARE
    v_balance INTEGER;
BEGIN
    SELECT balance INTO v_balance
    FROM subscriptions.credits
    WHERE user_id = p_user_id;

    RETURN COALESCE(v_balance, 0);
END;
$$ LANGUAGE plpgsql;

-- Function to add credits (purchase or refund)
CREATE OR REPLACE FUNCTION subscriptions.add_credits(
    p_user_id UUID,
    p_amount INTEGER,
    p_transaction_type VARCHAR(50),
    p_reason VARCHAR(255) DEFAULT NULL,
    p_reference_id UUID DEFAULT NULL,
    p_reference_type VARCHAR(50) DEFAULT NULL
)
RETURNS UUID AS $$
DECLARE
    v_transaction_id UUID;
    v_new_balance INTEGER;
BEGIN
    -- Insert or update user credits
    INSERT INTO subscriptions.credits (user_id, balance, total_purchased)
    VALUES (p_user_id, p_amount, p_amount)
    ON CONFLICT (user_id)
    DO UPDATE SET
        balance = subscriptions.credits.balance + p_amount,
        total_purchased = subscriptions.credits.total_purchased + p_amount,
        updated_at = NOW();

    -- Get new balance
    SELECT balance INTO v_new_balance
    FROM subscriptions.credits
    WHERE user_id = p_user_id;

    -- Record transaction
    INSERT INTO subscriptions.credit_transactions (
        user_id, amount, balance_after, transaction_type,
        reason, reference_id, reference_type
    ) VALUES (
        p_user_id, p_amount, v_new_balance, p_transaction_type,
        p_reason, p_reference_id, p_reference_type
    ) RETURNING id INTO v_transaction_id;

    RETURN v_transaction_id;
END;
$$ LANGUAGE plpgsql;

-- Function to consume credits (book/content generation)
CREATE OR REPLACE FUNCTION subscriptions.consume_credits(
    p_user_id UUID,
    p_amount INTEGER,
    p_reason VARCHAR(255),
    p_reference_id UUID DEFAULT NULL,
    p_reference_type VARCHAR(50) DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE
    v_current_balance INTEGER;
    v_new_balance INTEGER;
BEGIN
    -- Get current balance
    SELECT balance INTO v_current_balance
    FROM subscriptions.credits
    WHERE user_id = p_user_id
    FOR UPDATE;

    -- Check if sufficient credits
    IF v_current_balance IS NULL OR v_current_balance < p_amount THEN
        RETURN FALSE;
    END IF;

    -- Deduct credits
    UPDATE subscriptions.credits
    SET balance = balance - p_amount,
        total_consumed = total_consumed + p_amount,
        updated_at = NOW()
    WHERE user_id = p_user_id
    RETURNING balance INTO v_new_balance;

    -- Record transaction
    INSERT INTO subscriptions.credit_transactions (
        user_id, amount, balance_after, transaction_type,
        reason, reference_id, reference_type
    ) VALUES (
        p_user_id, -p_amount, v_new_balance, 'consumption',
        p_reason, p_reference_id, p_reference_type
    );

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Function to check if user has sufficient credits
CREATE OR REPLACE FUNCTION subscriptions.has_sufficient_credits(
    p_user_id UUID,
    p_required_amount INTEGER
)
RETURNS BOOLEAN AS $$
DECLARE
    v_balance INTEGER;
BEGIN
    SELECT balance INTO v_balance
    FROM subscriptions.credits
    WHERE user_id = p_user_id;

    RETURN COALESCE(v_balance, 0) >= p_required_amount;
END;
$$ LANGUAGE plpgsql;

--=============================================================================
-- VIEWS: REPORTING & ANALYTICS
--=============================================================================

-- View for user credit summary
CREATE OR REPLACE VIEW subscriptions.v_user_credit_summary AS
SELECT
    u.id AS user_id,
    u.email,
    COALESCE(c.balance, 0) AS current_balance,
    COALESCE(c.total_purchased, 0) AS lifetime_purchased,
    COALESCE(c.total_consumed, 0) AS lifetime_consumed,
    (
        SELECT COUNT(*)
        FROM subscriptions.credit_orders
        WHERE user_id = u.id AND status = 'completed'
    ) AS total_purchases,
    c.updated_at AS last_activity
FROM users.users u
LEFT JOIN subscriptions.credits c ON u.id = c.user_id;

-- View for credit package sales analytics
CREATE OR REPLACE VIEW subscriptions.v_credit_package_sales AS
SELECT
    cp.id AS package_id,
    cp.name,
    cp.credit_amount,
    cp.price_cents,
    COUNT(co.id) AS total_sales,
    SUM(CASE WHEN co.status = 'completed' THEN 1 ELSE 0 END) AS completed_sales,
    SUM(CASE WHEN co.status = 'completed' THEN co.price_cents ELSE 0 END) AS total_revenue_cents,
    SUM(CASE WHEN co.status = 'completed' THEN co.credit_amount ELSE 0 END) AS total_credits_sold
FROM subscriptions.credit_packages cp
LEFT JOIN subscriptions.credit_orders co ON cp.id = co.package_id
GROUP BY cp.id, cp.name, cp.credit_amount, cp.price_cents;

--=============================================================================
-- GRANTS (if using specific application user)
--=============================================================================

-- Grant permissions to application user (adjust role name as needed)
-- GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA subscriptions TO authorworks_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA subscriptions TO authorworks_app;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA subscriptions TO authorworks_app;

--=============================================================================
-- MIGRATION COMPLETE
--=============================================================================

-- Log migration completion
DO $$
BEGIN
    RAISE NOTICE 'Migration 001_add_credit_system.sql completed successfully';
END $$;
