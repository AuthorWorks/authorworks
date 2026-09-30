import type { Pool } from 'pg'

/**
 * Admission configuration. Nothing here invents prices, allowances or seller
 * configuration: allowances live per-user in ai_entitlements (provisioned by
 * the billing lane), and platform funding is OFF unless explicitly enabled.
 *
 * Env vars:
 *   AI_ADMISSION_ENABLED            'true' to allow any admission at all
 *                                    (kill switch / fail-closed default OFF).
 *   AI_ADMISSION_MAX_GLOBAL_ACTIVE   fleet-wide in-flight reservations
 *                                    (default 3; safety bound, not an allowance).
 *   AI_ADMISSION_RESERVATION_TTL_S   after this a live reservation becomes
 *                                    reconciled to 'unknown' (default 900).
 *   AI_ADMISSION_RETRY_AFTER_S       Retry-After for concurrency limits
 *                                    (default 5).
 */

export interface AdmissionConfig {
  enabled: boolean
  maxGlobalActive: number
  reservationTtlSeconds: number
  concurrencyRetryAfterSeconds: number
}

export function loadAdmissionConfig(env: NodeJS.ProcessEnv = process.env): AdmissionConfig {
  const int = (name: string, fallback: number, min: number): number => {
    const raw = env[name]
    if (raw === undefined || raw.trim() === '') return fallback
    const n = Number(raw)
    return Number.isFinite(n) && n >= min ? Math.floor(n) : fallback
  }
  return {
    enabled: env.AI_ADMISSION_ENABLED === 'true',
    maxGlobalActive: int('AI_ADMISSION_MAX_GLOBAL_ACTIVE', 3, 1),
    reservationTtlSeconds: int('AI_ADMISSION_RESERVATION_TTL_S', 900, 1),
    concurrencyRetryAfterSeconds: int('AI_ADMISSION_RETRY_AFTER_S', 5, 1),
  }
}

/**
 * Additive, idempotent schema (kept in lockstep with
 * migrations/2026-09-30-ai-admission.sql, which is the reviewed artifact for
 * out-of-band application). VARCHAR Logto subjects, not UUIDs.
 */
export const ADMISSION_SCHEMA_SQL = `
  CREATE TABLE IF NOT EXISTS public.ai_entitlements (
      user_id                              VARCHAR(255) PRIMARY KEY,
      plan_id                              VARCHAR(50)  NOT NULL,
      status                               VARCHAR(50)  NOT NULL DEFAULT 'active'
                                           CHECK (status IN ('active', 'paused', 'canceled')),
      monthly_output_token_allowance       BIGINT       NOT NULL CHECK (monthly_output_token_allowance >= 0),
      max_output_tokens_per_request        BIGINT       NOT NULL CHECK (max_output_tokens_per_request > 0),
      max_concurrent_generations           INTEGER      NOT NULL CHECK (max_concurrent_generations >= 0),
      period_start                         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
      created_at                           TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
      updated_at                           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
  );

  CREATE TABLE IF NOT EXISTS public.ai_reservations (
      id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
      user_id                  VARCHAR(255) NOT NULL,
      request_id               VARCHAR(200) NOT NULL,
      route                    VARCHAR(50)  NOT NULL,
      status                   VARCHAR(20)  NOT NULL DEFAULT 'reserved'
                               CHECK (status IN ('reserved', 'settled', 'released', 'unknown')),
      reserved_output_tokens  BIGINT       NOT NULL CHECK (reserved_output_tokens > 0),
      settled_input_tokens    BIGINT,
      settled_output_tokens   BIGINT,
      model                    TEXT,
      response_payload        JSONB,
      resolution_note         TEXT,
      created_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
      updated_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
      settled_at               TIMESTAMPTZ,
      UNIQUE (user_id, request_id)
  );

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
`

// Tracked per Pool: schema DDL is idempotent, but caching across unrelated
// pools (e.g. tests, multiple runtimes) would skip necessary CREATEs.
const schemasEnsured = new WeakSet<object>()

export async function ensureAdmissionSchema(pool: Pool): Promise<void> {
  if (schemasEnsured.has(pool)) return
  await pool.query('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
  await pool.query(ADMISSION_SCHEMA_SQL)
  schemasEnsured.add(pool)
}
