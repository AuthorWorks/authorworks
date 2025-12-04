-- AuthorWorks Database Initialization Script
-- This script creates the necessary databases and schemas

-- Create databases
CREATE DATABASE IF NOT EXISTS logto;
CREATE DATABASE IF NOT EXISTS authorworks;

-- Connect to authorworks database
\c authorworks;

-- Users schema
CREATE SCHEMA IF NOT EXISTS users;

-- Users table
CREATE TABLE IF NOT EXISTS users.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    password_hash VARCHAR(255),
    auth_provider VARCHAR(20),
    auth_provider_user_id VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'unverified',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE
);

-- Profiles table
CREATE TABLE IF NOT EXISTS users.profiles (
    user_id UUID PRIMARY KEY REFERENCES users.users(id) ON DELETE CASCADE,
    bio TEXT,
    avatar_url VARCHAR(255),
    website VARCHAR(255),
    location VARCHAR(255),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Social links table
CREATE TABLE IF NOT EXISTS users.social_links (
    user_id UUID REFERENCES users.users(id) ON DELETE CASCADE,
    platform VARCHAR(50) NOT NULL,
    url VARCHAR(255) NOT NULL,
    PRIMARY KEY (user_id, platform)
);

-- User roles table
CREATE TABLE IF NOT EXISTS users.user_roles (
    user_id UUID REFERENCES users.users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    PRIMARY KEY (user_id, role)
);

-- User preferences table
CREATE TABLE IF NOT EXISTS users.user_preferences (
    user_id UUID REFERENCES users.users(id) ON DELETE CASCADE,
    key VARCHAR(100) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, key)
);

-- Content schema
CREATE SCHEMA IF NOT EXISTS content;

-- Books table
CREATE TABLE IF NOT EXISTS content.books (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    genre VARCHAR(50),
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    cover_image_url VARCHAR(255),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    published_at TIMESTAMP WITH TIME ZONE
);

-- Chapters table
CREATE TABLE IF NOT EXISTS content.chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES content.books(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    chapter_number INTEGER NOT NULL,
    word_count INTEGER DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Subscriptions schema
CREATE SCHEMA IF NOT EXISTS subscriptions;

-- Subscription plans table
CREATE TABLE IF NOT EXISTS subscriptions.plans (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    price_cents INTEGER NOT NULL,
    interval VARCHAR(20) NOT NULL DEFAULT 'month',
    features JSONB DEFAULT '[]',
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- User subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions.user_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    plan_id VARCHAR(50) NOT NULL REFERENCES subscriptions.plans(id),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    stripe_subscription_id VARCHAR(255),
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    cancel_at_period_end BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    cancelled_at TIMESTAMP WITH TIME ZONE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_users_email ON users.users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users.users(username);
CREATE INDEX IF NOT EXISTS idx_users_auth_provider ON users.users(auth_provider, auth_provider_user_id) WHERE auth_provider IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_books_author ON content.books(author_id);
CREATE INDEX IF NOT EXISTS idx_books_status ON content.books(status);
CREATE INDEX IF NOT EXISTS idx_chapters_book ON content.chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions.user_subscriptions(user_id);

-- Insert default subscription plans
INSERT INTO subscriptions.plans (id, name, description, price_cents, interval, features)
VALUES 
    ('free', 'Free', 'Basic access to AuthorWorks', 0, 'month', '["5 books", "Basic editor", "Community support"]'),
    ('creator', 'Creator', 'For serious content creators', 999, 'month', '["Unlimited books", "Advanced editor", "AI assistance", "Priority support"]'),
    ('pro', 'Professional', 'For professional authors', 2999, 'month', '["Everything in Creator", "Team collaboration", "Publishing tools", "Analytics", "Dedicated support"]')
ON CONFLICT (id) DO NOTHING;

-- Grant permissions (adjust as needed for your setup)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA users TO authorworks;
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA content TO authorworks;
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA subscriptions TO authorworks;

COMMENT ON DATABASE authorworks IS 'AuthorWorks platform database';

