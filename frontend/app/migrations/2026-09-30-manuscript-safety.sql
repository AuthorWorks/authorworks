-- Manuscript-safe completion (fix/manuscript-safe-completion)
--
-- Adds the durable per-job generation application record used to make job
-- completion idempotent and owner/book-bound, plus optional generation_logs
-- columns the completion path writes (all statements are idempotent and
-- safe to re-run; they do NOT touch existing rows' data).

CREATE TABLE IF NOT EXISTS generation_completions (
    id            BIGSERIAL PRIMARY KEY,
    job_id        TEXT NOT NULL,
    book_id       UUID NOT NULL,
    user_id       VARCHAR(255) NOT NULL,
    status        TEXT NOT NULL DEFAULT 'applying',
    result        JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at  TIMESTAMPTZ,
    -- One job may be applied exactly once, globally.
    CONSTRAINT generation_completions_job_id_key UNIQUE (job_id)
);

CREATE INDEX IF NOT EXISTS idx_generation_completions_book_id
    ON generation_completions(book_id);

ALTER TABLE generation_completions
    ADD COLUMN IF NOT EXISTS error TEXT;

-- Columns the completion path updates on the existing generation_logs table.
-- (generation_logs already exists in production with book_id, generation_type,
-- status, result JSONB and timestamps; these additions are no-ops when present.)
ALTER TABLE generation_logs
    ADD COLUMN IF NOT EXISTS prompt       TEXT,
    ADD COLUMN IF NOT EXISTS error        TEXT,
    ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_generation_logs_book_id
    ON generation_logs(book_id);

-- No changes to books/chapters: the completion path stores its baseline and
-- pending-proposal data inside books.metadata (JSONB) and reuses the existing
-- chapter columns, so no data migration or backfill is required.
