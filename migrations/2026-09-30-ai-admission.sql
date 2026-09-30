-- AuthorWorks AI admission & usage accounting (additive)
-- Runtime tables use VARCHAR Logto subjects (matches public.books.user_id),
-- NOT the undeployed Rust subscription service's UUID schema.
-- Deployed via ensureAdmissionSchema() at runtime; kept as a file for review
-- and for operators who apply migrations out-of-band. Additive only.

-- Per-user entitlement row. Absence of a row means "not provisioned":
-- admission fails closed until billing (lane D) provisions one.
CREATE TABLE IF NOT EXISTS public.ai_entitlements (
    user_id                              VARCHAR(255) PRIMARY KEY,
    plan_id                              VARCHAR(50)  NOT NULL,
    status                               VARCHAR(50)  NOT NULL DEFAULT 'active'
                                         CHECK (status IN ('active', 'paused', 'canceled')),
    monthly_output_token_allowance       BIGINT       NOT NULL CHECK (monthly_output_token_allowance >= 0),
    max_output_tokens_per_request        BIGINT       NOT NULL CHECK (max_output_tokens_per_request > 0),
    max_concurrent_generations           INTEGER      NOT NULL CHECK (max_concurrent_generations >= 0),
    -- Start of the current metering period. Resettling the period is an
    -- explicit billing operation (UPDATE ... WHERE period_start < now() - interval '1 month'),
    -- never done implicitly by admission.
    period_start                         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_at                           TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at                           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Durable reservation for one paid inference request. (user_id, request_id)
-- is the idempotency key: a retried POST must find this row, never start a
-- second paid request.
CREATE TABLE IF NOT EXISTS public.ai_reservations (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  VARCHAR(255) NOT NULL,
    request_id               VARCHAR(200) NOT NULL,
    route                    VARCHAR(50)  NOT NULL, -- outline | enhance | book
    -- reserved: admitted, execution may still be running
    -- settled:  usage recorded exactly once (ai_usage_events row exists)
    -- released: admission cancelled before spend, frees hold
    -- unknown:  reservation outlived its TTL while spend state is uncertain;
    --           still holds quota; an operator must resolve it explicitly.
    status                   VARCHAR(20)  NOT NULL DEFAULT 'reserved'
                             CHECK (status IN ('reserved', 'settled', 'released', 'unknown')),
    reserved_output_tokens  BIGINT       NOT NULL CHECK (reserved_output_tokens > 0),
    settled_input_tokens    BIGINT,
    settled_output_tokens   BIGINT,
    model                    TEXT,
    -- Response payload captured at settlement so a duplicate admission can
    -- return the same job/response instead of re-running the paid request.
    response_payload        JSONB,
    resolution_note         TEXT,
    created_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    settled_at              TIMESTAMPTZ,
    UNIQUE (user_id, request_id)
);

-- Immutable usage ledger; exactly one row per settled reservation.
CREATE TABLE IF NOT EXISTS public.ai_usage_events (
    id               BIGSERIAL PRIMARY KEY,
    reservation_id   UUID NOT NULL UNIQUE REFERENCES public.ai_reservations (id) ON DELETE RESTRICT,
    user_id          VARCHAR(255) NOT NULL,
    route            VARCHAR(50)  NOT NULL,
    model            TEXT,
    input_tokens     BIGINT NOT NULL CHECK (input_tokens >= 0),
    output_tokens    BIGINT NOT NULL CHECK (output_tokens >= 0),
    period_start     TIMESTAMPTZ NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS ai_reservations_user_status_idx
    ON public.ai_reservations (user_id, status, created_at);
CREATE INDEX IF NOT EXISTS ai_usage_events_user_period_idx
    ON public.ai_usage_events (user_id, period_start, created_at);
