-- Migration: 003 - Per-user AI provider settings (bring-your-own-key / inference)
-- Description: stores each user's choice of inference backend.
--   provider: 'default' (platform LiteLLM gateway) | 'openai-compatible'
--   api_key_encrypted: AES-256-GCM, encrypted by the frontend with
--     AI_KEY_ENCRYPTION_SECRET (see frontend/app/src/app/lib/crypto.ts)
-- Run after 002_frontend_schema.sql. Idempotent.

CREATE TABLE IF NOT EXISTS public.user_ai_settings (
    user_id           VARCHAR(255) PRIMARY KEY,
    provider          VARCHAR(50) NOT NULL DEFAULT 'default',
    base_url          TEXT,
    model             TEXT,
    api_key_encrypted TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Frontend also self-heals this table at runtime, but keep schema versioned here.
CREATE TABLE IF NOT EXISTS public.user_profiles (
    user_id    VARCHAR(255) PRIMARY KEY,
    name       TEXT,
    bio        TEXT,
    website    TEXT,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
