-- Migration: 002 - Frontend app tables (Next.js API)
-- Description: generation_logs for AI job tracking; optional public.books/chapters for public-only DB
-- Run after 001_add_credit_system.sql when content schema exists; safe when content.books/chapters already exist

-- Frontend expects generation_logs for outline, chapter gen, and full-book job tracking
CREATE TABLE IF NOT EXISTS public.generation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL,
    generation_type VARCHAR(50) NOT NULL,
    prompt TEXT,
    model VARCHAR(100),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    result JSONB,
    error TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_generation_logs_book_id ON public.generation_logs(book_id);
CREATE INDEX IF NOT EXISTS idx_generation_logs_status ON public.generation_logs(status);
CREATE INDEX IF NOT EXISTS idx_generation_logs_created_at ON public.generation_logs(created_at DESC);

-- Optional: public.books / public.chapters for deployments that do not use content schema
-- Skip if you use content.books and content.chapters only
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'books') THEN
        CREATE TABLE public.books (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id VARCHAR(255) NOT NULL,
            title VARCHAR(500) NOT NULL,
            description TEXT,
            genre VARCHAR(100),
            status VARCHAR(50) DEFAULT 'draft',
            word_count INTEGER DEFAULT 0,
            metadata JSONB DEFAULT '{}',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        CREATE INDEX idx_books_user_id ON public.books(user_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'chapters') THEN
        CREATE TABLE public.chapters (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            book_id UUID NOT NULL,
            chapter_number INTEGER NOT NULL,
            title VARCHAR(500) NOT NULL,
            content TEXT,
            word_count INTEGER DEFAULT 0,
            status VARCHAR(50) DEFAULT 'draft',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        CREATE INDEX idx_chapters_book_id ON public.chapters(book_id);
    END IF;
END $$;
