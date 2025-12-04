-- AuthorWorks Database Schema
-- PostgreSQL 15+

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create schemas
CREATE SCHEMA IF NOT EXISTS users;
CREATE SCHEMA IF NOT EXISTS content;
CREATE SCHEMA IF NOT EXISTS storage;
CREATE SCHEMA IF NOT EXISTS editor;
CREATE SCHEMA IF NOT EXISTS subscriptions;
CREATE SCHEMA IF NOT EXISTS messaging;
CREATE SCHEMA IF NOT EXISTS discovery;
CREATE SCHEMA IF NOT EXISTS media;

--=============================================================================
-- USERS SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS users.users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    logto_id VARCHAR(255) UNIQUE,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    avatar_url TEXT,
    bio TEXT,
    status VARCHAR(50) DEFAULT 'active',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS users.profiles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    display_name VARCHAR(255),
    website VARCHAR(500),
    social_links JSONB DEFAULT '{}',
    preferences JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users.users(email);
CREATE INDEX IF NOT EXISTS idx_users_logto_id ON users.users(logto_id);

--=============================================================================
-- CONTENT SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS content.books (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    author_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    genre VARCHAR(100),
    status VARCHAR(50) DEFAULT 'draft',
    cover_image_url TEXT,
    word_count INTEGER DEFAULT 0,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    published_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS content.chapters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    book_id UUID NOT NULL REFERENCES content.books(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    content TEXT,
    chapter_number INTEGER NOT NULL,
    word_count INTEGER DEFAULT 0,
    status VARCHAR(50) DEFAULT 'draft',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS content.scenes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chapter_id UUID NOT NULL REFERENCES content.chapters(id) ON DELETE CASCADE,
    title VARCHAR(500),
    content TEXT,
    scene_number INTEGER NOT NULL,
    word_count INTEGER DEFAULT 0,
    pov_character VARCHAR(255),
    location VARCHAR(255),
    time_period VARCHAR(255),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS content.characters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    book_id UUID NOT NULL REFERENCES content.books(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(100),
    description TEXT,
    backstory TEXT,
    traits JSONB DEFAULT '[]',
    relationships JSONB DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS content.generation_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    book_id UUID REFERENCES content.books(id) ON DELETE SET NULL,
    job_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    input JSONB NOT NULL,
    output JSONB,
    error TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_books_author ON content.books(author_id);
CREATE INDEX IF NOT EXISTS idx_books_status ON content.books(status);
CREATE INDEX IF NOT EXISTS idx_chapters_book ON content.chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_chapters_number ON content.chapters(book_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON content.generation_jobs(status);

--=============================================================================
-- STORAGE SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS storage.files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    filename VARCHAR(500) NOT NULL,
    s3_key VARCHAR(1000) NOT NULL,
    content_type VARCHAR(255) NOT NULL,
    size BIGINT NOT NULL,
    checksum VARCHAR(64),
    file_type VARCHAR(50),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_files_user ON storage.files(user_id);
CREATE INDEX IF NOT EXISTS idx_files_type ON storage.files(file_type);

--=============================================================================
-- EDITOR SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS editor.documents (
    id UUID PRIMARY KEY,
    content TEXT DEFAULT '',
    version BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS editor.operations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES editor.documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    operation JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS editor.checkpoints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES editor.documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    version BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS editor.presence (
    document_id UUID NOT NULL REFERENCES editor.documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    cursor_position INTEGER,
    selection_start INTEGER,
    selection_end INTEGER,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (document_id, user_id)
);

CREATE TABLE IF NOT EXISTS editor.comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES editor.documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    position_start INTEGER NOT NULL,
    position_end INTEGER NOT NULL,
    resolved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_operations_document ON editor.operations(document_id, version);
CREATE INDEX IF NOT EXISTS idx_checkpoints_document ON editor.checkpoints(document_id);
CREATE INDEX IF NOT EXISTS idx_presence_updated ON editor.presence(updated_at);

--=============================================================================
-- SUBSCRIPTIONS SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS subscriptions.subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    plan_id VARCHAR(50) NOT NULL DEFAULT 'free',
    status VARCHAR(50) DEFAULT 'active',
    stripe_subscription_id VARCHAR(255),
    stripe_customer_id VARCHAR(255),
    current_period_start TIMESTAMPTZ,
    current_period_end TIMESTAMPTZ,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE TABLE IF NOT EXISTS subscriptions.invoices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_customer_id VARCHAR(255) NOT NULL,
    stripe_invoice_id VARCHAR(255) NOT NULL,
    amount BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS subscriptions.ai_usage (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    word_count INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions.subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe ON subscriptions.subscriptions(stripe_subscription_id);
CREATE INDEX IF NOT EXISTS idx_ai_usage_user ON subscriptions.ai_usage(user_id, created_at);

--=============================================================================
-- MESSAGING SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS messaging.notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    data JSONB DEFAULT '{}',
    read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS messaging.conversations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255),
    type VARCHAR(50) DEFAULT 'direct',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS messaging.conversation_members (
    conversation_id UUID NOT NULL REFERENCES messaging.conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (conversation_id, user_id)
);

CREATE TABLE IF NOT EXISTS messaging.messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conversation_id UUID NOT NULL REFERENCES messaging.conversations(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    attachments JSONB DEFAULT '[]',
    read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS messaging.events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    delivered BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON messaging.notifications(user_id, read);
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messaging.messages(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_events_user ON messaging.events(user_id, delivered);

--=============================================================================
-- DISCOVERY SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS discovery.reading_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    book_id UUID NOT NULL REFERENCES content.books(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES content.chapters(id) ON DELETE SET NULL,
    progress FLOAT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS discovery.bookmarks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    book_id UUID NOT NULL REFERENCES content.books(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES content.chapters(id) ON DELETE SET NULL,
    position INTEGER,
    note TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reading_history_user ON discovery.reading_history(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_reading_history_book ON discovery.reading_history(book_id);

--=============================================================================
-- MEDIA SCHEMA
--=============================================================================

CREATE TABLE IF NOT EXISTS media.jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users.users(id) ON DELETE CASCADE,
    job_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    input JSONB NOT NULL,
    output JSONB,
    error TEXT,
    progress INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS media.job_queue (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID NOT NULL REFERENCES media.jobs(id) ON DELETE CASCADE,
    job_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS media.thumbnails (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_id UUID NOT NULL REFERENCES storage.files(id) ON DELETE CASCADE,
    thumbnail_file_id UUID REFERENCES storage.files(id) ON DELETE SET NULL,
    s3_key VARCHAR(1000) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(file_id)
);

CREATE INDEX IF NOT EXISTS idx_media_jobs_user ON media.jobs(user_id);
CREATE INDEX IF NOT EXISTS idx_media_jobs_status ON media.jobs(status);
CREATE INDEX IF NOT EXISTS idx_job_queue_processed ON media.job_queue(processed, created_at);

--=============================================================================
-- FUNCTIONS & TRIGGERS
--=============================================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers
CREATE OR REPLACE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users.users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE TRIGGER update_books_updated_at
    BEFORE UPDATE ON content.books
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE TRIGGER update_chapters_updated_at
    BEFORE UPDATE ON content.chapters
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE OR REPLACE TRIGGER update_subscriptions_updated_at
    BEFORE UPDATE ON subscriptions.subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Function to update book word count when chapters change
CREATE OR REPLACE FUNCTION update_book_word_count()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE content.books
    SET word_count = (
        SELECT COALESCE(SUM(word_count), 0)
        FROM content.chapters
        WHERE book_id = COALESCE(NEW.book_id, OLD.book_id)
    ),
    updated_at = NOW()
    WHERE id = COALESCE(NEW.book_id, OLD.book_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER update_book_word_count_trigger
    AFTER INSERT OR UPDATE OR DELETE ON content.chapters
    FOR EACH ROW EXECUTE FUNCTION update_book_word_count();

