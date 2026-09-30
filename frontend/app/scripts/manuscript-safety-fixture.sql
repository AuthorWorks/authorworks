-- Test fixture mirroring the LIVE AuthorWorks production schema (per operator
-- investigation 2026-09-30), deliberately NOT init-schema.sql:
--  - books.user_id is VARCHAR (no users FK)
--  - chapters has NO metadata column and NO UNIQUE(book_id, chapter_number)
--  - no triggers
CREATE TABLE books (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     VARCHAR(255) NOT NULL,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'draft',
    word_count  INTEGER NOT NULL DEFAULT 0,
    metadata    JSONB DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE chapters (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id        UUID NOT NULL,
    chapter_number INTEGER NOT NULL,
    title          TEXT,
    content        TEXT,
    word_count     INTEGER NOT NULL DEFAULT 0,
    status         TEXT NOT NULL DEFAULT 'draft',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE generation_logs (
    id              BIGSERIAL PRIMARY KEY,
    book_id         UUID NOT NULL,
    generation_type TEXT NOT NULL,
    prompt          TEXT,
    status          TEXT NOT NULL DEFAULT 'pending',
    result          JSONB,
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);
