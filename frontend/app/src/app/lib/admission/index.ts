import type { Pool, PoolClient } from 'pg'
import {
  AdmissionConfig,
  ensureAdmissionSchema,
  loadAdmissionConfig,
} from './config'
import { AdmissionError } from './errors'

/**
 * Transactional bounded-AI admission and usage accounting.
 *
 * Invariants (all enforced under a per-user pg_advisory_xact_lock so the
 * SELECT-then-INSERT race cannot overspend; a plain "INSERT ... WHERE
 * budget remains" under READ COMMITTED is NOT sufficient):
 *   1. (user_id, request_id) is the idempotency key — a duplicate admission
 *      returns the SAME reservation (and, if settled, its stored response),
 *      never starts a second paid request.
 *   2. Quota holds count BOTH settled usage and live reservations, so N
 *      concurrent admissions cannot collectively exceed the allowance.
 *   3. Settlement + usage-event creation are one transaction; exactly one
 *      usage event exists per reservation (UNIQUE constraint backstop).
 *   4. Settlement/release validate reservation ownership.
 *   5. A reservation that outlives its TTL becomes 'unknown' — it KEEPS
 *      holding quota, because a timed-out or missing job is not proof of
 *      zero spend. Only explicit operator resolution releases it.
 *   6. With AI_ADMISSION_ENABLED !== 'true' admission fails closed.
 */

export const VALID_ROUTES = ['outline', 'enhance', 'book'] as const
export type AdmissionRoute = (typeof VALID_ROUTES)[number]

export interface AdmitInput {
  userId: string
  /** Client-supplied idempotency key (durable per route). */
  requestId: string
  route: AdmissionRoute
  /** Output tokens the caller intends to request. */
  requestedOutputTokens: number
}

export interface AdmitResult {
  reservationId: string
  /** Current reservation state; 'released' only appears on duplicate replay. */
  status: 'reserved' | 'settled' | 'released' | 'unknown'
  /** True when this call matched an existing reservation (idempotent retry). */
  duplicate: boolean
  /** Present when the matched reservation was already settled. */
  responsePayload?: unknown
  /** Response payload captured by this call's settlement (undefined while reserved). */
  reservedOutputTokens: number
}

export interface SettleInput {
  reservationId: string
  userId: string
  model?: string | null
  inputTokens: number
  outputTokens: number
  /** Response to persist for idempotent replay (kept small). */
  responsePayload?: unknown
}

export interface SettleResult {
  reservationId: string
  duplicate: boolean
  responsePayload?: unknown
}

export interface UsageSnapshot {
  userId: string
  entitlement: {
    planId: string
    status: string
    monthlyOutputTokenAllowance: number
    maxOutputTokensPerRequest: number
    maxConcurrentGenerations: number
    periodStart: string
  } | null
  settledOutputTokens: number
  /** Tokens held by reserved/unknown reservations. */
  heldOutputTokens: number
  activeReservations: number
  remainingOutputTokens: number | null
}

// Advisory lock keys. Per-user: hashed from the user id. Global: fixed.
const GLOBAL_LOCK_KEY = 0x41a01a

export class Admission {
  private readonly pool: Pool
  private readonly config: AdmissionConfig

  constructor(pool: Pool, config: AdmissionConfig = loadAdmissionConfig()) {
    this.pool = pool
    this.config = config
  }

  async init(): Promise<void> {
    await ensureAdmissionSchema(this.pool)
  }

  /**
   * Atomically decide admission for one paid request. Either returns a
   * reservation (new or existing) or throws a typed AdmissionError.
   */
  async admit(input: AdmitInput): Promise<AdmitResult> {
    if (!this.config.enabled) {
      throw new AdmissionError(
        'admission_disabled',
        'AI generation is not configured on this deployment yet. Your writing, editing and export are unaffected. Please try again later or contact support.',
        { reason: 'admission_not_configured' }
      )
    }
    if (!input.userId || !input.requestId || !VALID_ROUTES.includes(input.route)) {
      throw new AdmissionError('invalid_request', 'userId, requestId and a valid route are required')
    }
    if (!Number.isFinite(input.requestedOutputTokens) || input.requestedOutputTokens <= 0) {
      throw new AdmissionError('invalid_request', 'requestedOutputTokens must be a positive integer')
    }

    const client = await this.pool.connect()
    try {
      await client.query('BEGIN')
      // Per-user serialization: all admission decisions for this user are
      // linearized, closing the check-then-insert race.
      await client.query('SELECT pg_advisory_xact_lock(hashtext($1))', [input.userId])

      // --- Idempotency: existing reservation for this (user, request_id) ---
      const existing = await client.query(
        `SELECT id, status, reserved_output_tokens, response_payload
           FROM public.ai_reservations
          WHERE user_id = $1 AND request_id = $2
          FOR UPDATE`,
        [input.userId, input.requestId]
      )
      if (existing.rows.length > 0) {
        const row = existing.rows[0]
        await client.query('COMMIT')
        // Any status here means the idempotency key is consumed: the caller
        // must NOT start another paid request. Settled duplicates replay the
        // stored response; reserved/unknown are still in flight; released
        // consumed the key without spend (route surfaces a 409).
        return {
          reservationId: row.id,
          status: row.status as AdmitResult['status'],
          duplicate: true,
          ...(row.status === 'settled' && { responsePayload: row.response_payload }),
          reservedOutputTokens: Number(row.reserved_output_tokens),
        }
      }

      // --- Entitlement (must exist and be active; fail closed otherwise) ---
      const ent = await client.query(
        `SELECT plan_id, status, monthly_output_token_allowance,
                max_output_tokens_per_request, max_concurrent_generations, period_start
           FROM public.ai_entitlements
          WHERE user_id = $1
          FOR UPDATE`,
        [input.userId]
      )
      if (ent.rows.length === 0) {
        throw new AdmissionError(
          'entitlement_missing',
          'You do not have an AI plan on this account yet. Subscribe or contact support to enable AI generation. Your books, editing and export remain fully available.',
          { reason: 'no_entitlement' }
        )
      }
      const e = ent.rows[0]
      if (e.status !== 'active') {
        throw new AdmissionError(
          'entitlement_missing',
          `Your plan (${e.plan_id}) is ${e.status}. AI generation is unavailable until it is reactivated. Editing and export remain fully available.`,
          { reason: 'entitlement_inactive' }
        )
      }

      const maxPerRequest = Number(e.max_output_tokens_per_request)
      if (input.requestedOutputTokens > maxPerRequest) {
        throw new AdmissionError(
          'invalid_request',
          `This request asks for ${input.requestedOutputTokens} output tokens but your plan caps requests at ${maxPerRequest}. Reduce the request size.`,
          { reason: 'per_request_limit' }
        )
      }

      // --- Concurrency (per-user, then global) ---
      const active = await client.query(
        `SELECT COUNT(*) AS n
           FROM public.ai_reservations
          WHERE user_id = $1 AND status IN ('reserved', 'unknown')`,
        [input.userId]
      )
      const activeCount = Number(active.rows[0].n)
      if (activeCount >= Number(e.max_concurrent_generations)) {
        throw new AdmissionError(
          'concurrency_limit',
          `You already have ${activeCount} generation(s) in progress. Wait for one to finish before starting another.`,
          { retryAfterSeconds: this.config.concurrencyRetryAfterSeconds }
        )
      }

      // Global bound. Serialized by its own advisory lock so the count is exact.
      await client.query('SELECT pg_advisory_xact_lock($1)', [GLOBAL_LOCK_KEY])
      const globalActive = await client.query(
        `SELECT COUNT(*) AS n FROM public.ai_reservations WHERE status IN ('reserved', 'unknown')`
      )
      if (Number(globalActive.rows[0].n) >= this.config.maxGlobalActive) {
        throw new AdmissionError(
          'concurrency_limit',
          'The platform is at capacity for AI generation right now. Please retry shortly.',
          { retryAfterSeconds: this.config.concurrencyRetryAfterSeconds, reason: 'global_capacity' }
        )
      }

      // --- Quota: settled usage + live holds + this request <= allowance ---
      const usage = await client.query(
        `SELECT
           (SELECT COALESCE(SUM(u.output_tokens), 0)
              FROM public.ai_usage_events u
             WHERE u.user_id = $1 AND u.created_at >= $2::timestamptz) AS settled,
           (SELECT COALESCE(SUM(r.reserved_output_tokens), 0)
              FROM public.ai_reservations r
             WHERE r.user_id = $1 AND r.status IN ('reserved', 'unknown')) AS held`,
        [input.userId, e.period_start]
      )
      const settled = Number(usage.rows[0].settled)
      const held = Number(usage.rows[0].held)
      const allowance = Number(e.monthly_output_token_allowance)
      if (settled + held + input.requestedOutputTokens > allowance) {
        // Period reset (billing-lane operation) is the only retryable horizon.
        const retryAfter = this.config.reservationTtlSeconds
        throw new AdmissionError(
          'quota_exceeded',
          `This request would exceed your monthly allowance of ${allowance} output tokens (${settled} used, ${held} in flight). It resets with your billing period.`,
          { retryAfterSeconds: retryAfter, reason: 'monthly_allowance' }
        )
      }

      // --- Reserve ---
      const ins = await client.query(
        `INSERT INTO public.ai_reservations
           (user_id, request_id, route, status, reserved_output_tokens)
         VALUES ($1, $2, $3, 'reserved', $4)
         RETURNING id`,
        [input.userId, input.requestId, input.route, input.requestedOutputTokens]
      )
      await client.query('COMMIT')
      return {
        reservationId: ins.rows[0].id,
        status: 'reserved',
        duplicate: false,
        reservedOutputTokens: input.requestedOutputTokens,
      }
    } catch (error) {
      await client.query('ROLLBACK').catch(() => {})
      throw error
    } finally {
      client.release()
    }
  }

  /**
   * Settle a reservation: record usage and capture the response atomically.
   * Idempotent: re-settling returns the stored payload without creating a
   * second usage event. Validates ownership.
   */
  async settle(input: SettleInput): Promise<SettleResult> {
    if (
      !Number.isFinite(input.inputTokens) ||
      input.inputTokens < 0 ||
      !Number.isFinite(input.outputTokens) ||
      input.outputTokens < 0
    ) {
      throw new AdmissionError('invalid_request', 'inputTokens/outputTokens must be non-negative integers')
    }
    const client = await this.pool.connect()
    try {
      await client.query('BEGIN')
      const upd = await client.query(
        `UPDATE public.ai_reservations
            SET status = 'settled',
                settled_input_tokens = $3,
                settled_output_tokens = $4,
                model = COALESCE($5, model),
                response_payload = COALESCE($6, response_payload),
                settled_at = NOW(),
                updated_at = NOW()
          WHERE id = $1 AND user_id = $2 AND status = 'reserved'
          RETURNING id, response_payload`,
        [
          input.reservationId,
          input.userId,
          input.inputTokens,
          input.outputTokens,
          input.model ?? null,
          input.responsePayload !== undefined ? JSON.stringify(input.responsePayload) : null,
        ]
      )
      if (upd.rows.length === 0) {
        const cur = await client.query(
          `SELECT user_id, status, response_payload FROM public.ai_reservations WHERE id = $1`,
          [input.reservationId]
        )
        if (cur.rows.length === 0) {
          throw new AdmissionError('invalid_request', 'Unknown reservation', {
            reason: 'reservation_not_found',
          })
        }
        const row = cur.rows[0]
        if (row.user_id !== input.userId) {
          throw new AdmissionError('ownership', 'Reservation belongs to another user')
        }
        // Already settled (or released/unknown) — replay stored state.
        await client.query('COMMIT')
        return {
          reservationId: input.reservationId,
          duplicate: true,
          ...(row.status === 'settled' && row.response_payload !== null && {
            responsePayload: row.response_payload,
          }),
        }
      }
      // Exactly-once usage event; UNIQUE(reservation_id) is the backstop.
      await client.query(
        `INSERT INTO public.ai_usage_events
           (reservation_id, user_id, route, model, input_tokens, output_tokens, period_start)
         SELECT r.id, r.user_id, r.route, r.model, $3, $4,
                COALESCE(pe.period_start, r.created_at)
           FROM public.ai_reservations r
           LEFT JOIN public.ai_entitlements pe ON pe.user_id = r.user_id
          WHERE r.id = $1 AND r.user_id = $2
         ON CONFLICT (reservation_id) DO NOTHING`,
        [input.reservationId, input.userId, input.inputTokens, input.outputTokens]
      )
      await client.query('COMMIT')
      return {
        reservationId: input.reservationId,
        duplicate: false,
        ...(input.responsePayload !== undefined && { responsePayload: input.responsePayload }),
      }
    } catch (error) {
      await client.query('ROLLBACK').catch(() => {})
      throw error
    } finally {
      client.release()
    }
  }

  /**
   * Release a reservation that never spent (caller-sure failure BEFORE any
   * inference call was made, e.g. validation failure after admission).
   * Frees the hold. Does NOT delete the reservation — the idempotency key
   * stays consumed so a retry of the same request does not restart spend
   * under an ambiguous execution.
   */
  async release(input: {
    reservationId: string
    userId: string
    reason?: string
  }): Promise<void> {
    const client = await this.pool.connect()
    try {
      await client.query('BEGIN')
      const upd = await client.query(
        `UPDATE public.ai_reservations
            SET status = 'released',
                resolution_note = $3,
                updated_at = NOW()
          WHERE id = $1 AND user_id = $2 AND status = 'reserved'`,
        [input.reservationId, input.userId, input.reason ?? null]
      )
      if (upd.rowCount === 0) {
        const cur = await client.query(
          `SELECT user_id FROM public.ai_reservations WHERE id = $1`,
          [input.reservationId]
        )
        await client.query('COMMIT')
        if (cur.rows.length > 0 && cur.rows[0].user_id !== input.userId) {
          throw new AdmissionError('ownership', 'Reservation belongs to another user')
        }
        return // already settled/released/unknown — idempotent no-op
      }
      await client.query('COMMIT')
    } catch (error) {
      await client.query('ROLLBACK').catch(() => {})
      throw error
    } finally {
      client.release()
    }
  }

  /**
   * Restart/periodic reconciliation. Marks reservations that outlived the TTL
   * as 'unknown'. They KEEP holding quota (a timed-out or missing job is not
   * proof of zero spend); an operator must later resolve them via
   * resolveUnknown. Safe to run repeatedly; returns the ids processed.
   */
  async reconcileUnknown(opts: { olderThanSeconds?: number } = {}): Promise<string[]> {
    const ttl = opts.olderThanSeconds ?? this.config.reservationTtlSeconds
    const res = await this.pool.query(
      `UPDATE public.ai_reservations
          SET status = 'unknown',
              resolution_note = 'TTL elapsed while execution state unknown; held pending operator resolution',
              updated_at = NOW()
        WHERE status = 'reserved'
          AND created_at < NOW() - ($1::text || ' seconds')::interval
        RETURNING id`,
      [ttl]
    )
    return res.rows.map((r) => r.id)
  }

  /**
   * Operator resolution of an unknown reservation AFTER confirming the real
   * spend state externally (provider logs). Either settle with actual tokens
   * or release with zero spend. This is the ONLY path out of 'unknown'.
   */
  async resolveUnknown(input: {
    reservationId: string
    userId: string
    outcome: 'spent' | 'not_spent'
    inputTokens?: number
    outputTokens?: number
    note?: string
  }): Promise<void> {
    if (input.outcome === 'spent') {
      if (input.outputTokens === undefined) {
        throw new AdmissionError('invalid_request', 'outputTokens required for outcome=spent')
      }
      // Flip unknown -> reserved then reuse the settle path atomically.
      const client = await this.pool.connect()
      try {
        await client.query('BEGIN')
        await client.query(
          `UPDATE public.ai_reservations
              SET status = 'reserved', updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'unknown'`,
          [input.reservationId, input.userId]
        )
        await client.query('COMMIT')
      } catch (error) {
        await client.query('ROLLBACK').catch(() => {})
        throw error
      } finally {
        client.release()
      }
      await this.settle({
        reservationId: input.reservationId,
        userId: input.userId,
        inputTokens: input.inputTokens ?? 0,
        outputTokens: input.outputTokens,
      })
    } else {
      const client = await this.pool.connect()
      try {
        await client.query('BEGIN')
        const upd = await client.query(
          `UPDATE public.ai_reservations
              SET status = 'released',
                  resolution_note = COALESCE($3, 'operator confirmed zero spend'),
                  updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'unknown'`,
          [input.reservationId, input.userId, input.note ?? null]
        )
        if (upd.rowCount === 0) {
          const cur = await client.query(
            `SELECT user_id FROM public.ai_reservations WHERE id = $1`,
            [input.reservationId]
          )
          await client.query('COMMIT')
          if (cur.rows.length > 0 && cur.rows[0].user_id !== input.userId) {
            throw new AdmissionError('ownership', 'Reservation belongs to another user')
          }
          return
        }
        await client.query('COMMIT')
      } catch (error) {
        await client.query('ROLLBACK').catch(() => {})
        throw error
      } finally {
        client.release()
      }
    }
  }

  /** Usage summary for the subscription/settings UI. Never blocks generation. */
  async getUsageSnapshot(userId: string): Promise<UsageSnapshot> {
    const entRow = await this.pool.query(
      `SELECT e.plan_id, e.status, e.monthly_output_token_allowance,
              e.max_output_tokens_per_request, e.max_concurrent_generations, e.period_start,
              (SELECT COALESCE(SUM(u.output_tokens), 0)
                 FROM public.ai_usage_events u
                WHERE u.user_id = $1 AND u.created_at >= e.period_start) AS settled,
              (SELECT COALESCE(SUM(r.reserved_output_tokens), 0)
                 FROM public.ai_reservations r
                WHERE r.user_id = $1 AND r.status IN ('reserved', 'unknown')) AS held,
              (SELECT COUNT(*)
                 FROM public.ai_reservations r
                WHERE r.user_id = $1 AND r.status IN ('reserved', 'unknown')) AS active
         FROM public.ai_entitlements e
        WHERE e.user_id = $1`,
      [userId]
    )
    if (entRow.rows.length === 0) {
      return {
        userId,
        entitlement: null,
        settledOutputTokens: 0,
        heldOutputTokens: 0,
        activeReservations: 0,
        remainingOutputTokens: null,
      }
    }
    const e = entRow.rows[0]
    const settled = Number(e.settled ?? 0)
    const held = Number(e.held ?? 0)
    const active = Number(e.active ?? 0)
    return {
      userId,
      entitlement: {
        planId: e.plan_id,
        status: e.status,
        monthlyOutputTokenAllowance: Number(e.monthly_output_token_allowance),
        maxOutputTokensPerRequest: Number(e.max_output_tokens_per_request),
        maxConcurrentGenerations: Number(e.max_concurrent_generations),
        periodStart: e.period_start,
      },
      settledOutputTokens: settled,
      heldOutputTokens: held,
      activeReservations: active,
      remainingOutputTokens: Math.max(
        0,
        Number(e.monthly_output_token_allowance) - settled - held
      ),
    }
  }
}

/** Convenience: reserve tokens for one generation call, honoring the repo Pool. */
export function getAdmission(pool: Pool, config?: AdmissionConfig): Admission {
  return new Admission(pool, config)
}
