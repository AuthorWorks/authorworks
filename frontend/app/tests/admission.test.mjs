// Isolated database tests for the AI admission/usage module.
// Runs against a throwaway Postgres (AUTHORWORKS_TEST_DATABASE_URL);
// creates its own schema and never touches shared databases.
//
//   node --test tests/admission.test.mjs

import { test } from 'node:test'
import assert from 'node:assert/strict'
import pg from 'pg'
import { getAdmission, Admission } from '../.hermes-build/admission-lib/index.js'

const DB_URL =
  process.env.AUTHORWORKS_TEST_DATABASE_URL ??
  'postgres://awtest@127.0.0.1:5544/aw_admission_test'

const mkConfig = (over = {}) => ({
  enabled: true,
  maxGlobalActive: 50,
  reservationTtlSeconds: 900,
  concurrencyRetryAfterSeconds: 5,
  ...over,
})

async function freshDb(t) {
  const pool = new pg.Pool({ connectionString: DB_URL, max: 20 })
  // Full isolation per test: drop and recreate the tables the module owns.
  await pool.query(`
    DROP TABLE IF EXISTS public.ai_usage_events, public.ai_reservations, public.ai_entitlements CASCADE
  `)
  const adm = getAdmission(pool, mkConfig())
  await adm.init()
  t.after(() => pool.end())
  return { pool, adm }
}

async function provision(pool, userId, over = {}) {
  await pool.query(
    `INSERT INTO public.ai_entitlements
       (user_id, plan_id, monthly_output_token_allowance,
        max_output_tokens_per_request, max_concurrent_generations)
     VALUES ($1, 'test-plan', $2, $3, $4)`,
    [
      userId,
      over.allowance ?? 10_000,
      over.maxPerRequest ?? 8_000,
      over.maxConcurrent ?? 3,
    ]
  )
}

test('admission disabled fails closed with actionable error', async (t) => {
  const { pool } = await freshDb(t)
  const adm = getAdmission(pool, mkConfig({ enabled: false }))
  await provision(pool, 'u1')
  await assert.rejects(
    adm.admit({ userId: 'u1', requestId: 'r1', route: 'enhance', requestedOutputTokens: 100 }),
    (e) =>
      e.kind === 'admission_disabled' &&
      e.status === 503 &&
      e.retryAfterSeconds === undefined &&
      /contact|try again/i.test(e.message)
  )
})

test('missing entitlement is distinct from quota/concurrency', async (t) => {
  const { adm } = await freshDb(t)
  await assert.rejects(
    adm.admit({ userId: 'nobody', requestId: 'r1', route: 'outline', requestedOutputTokens: 100 }),
    (e) => e.kind === 'entitlement_missing' && e.status === 402 && !e.retryAfterSeconds
  )
})

test('paused entitlement fails with entitlement reason', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1')
  await pool.query(`UPDATE public.ai_entitlements SET status = 'paused' WHERE user_id = 'u1'`)
  await assert.rejects(
    adm.admit({ userId: 'u1', requestId: 'r1', route: 'enhance', requestedOutputTokens: 10 }),
    (e) => e.kind === 'entitlement_missing' && e.reason === 'entitlement_inactive'
  )
})

test('per-request cap rejects as invalid_request (400, no retry)', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1', { maxPerRequest: 100 })
  await assert.rejects(
    adm.admit({ userId: 'u1', requestId: 'r1', route: 'enhance', requestedOutputTokens: 101 }),
    (e) => e.kind === 'invalid_request' && e.status === 400 && e.reason === 'per_request_limit'
  )
})

test('concurrent admissions cannot overspend the allowance', async (t) => {
  const { pool, adm } = await freshDb(t)
  const allowance = 1_000
  await provision(pool, 'u1', { allowance, maxConcurrent: 50 })
  const results = await Promise.allSettled(
    Array.from({ length: 20 }, (_, i) =>
      adm.admit({
        userId: 'u1',
        requestId: `r-${i}`,
        route: 'enhance',
        requestedOutputTokens: 100,
      })
    )
  )
  const ok = results.filter((r) => r.status === 'fulfilled')
  const quotaErrs = results.filter(
    (r) => r.status === 'rejected' && r.reason?.kind === 'quota_exceeded'
  )
  // 10 admissions of 100 tokens each exactly exhaust 1000; nothing more.
  assert.equal(ok.length, 10, `expected exactly 10 admissions, got ${ok.length}`)
  assert.equal(quotaErrs.length, 10)
  for (const r of quotaErrs) {
    assert.equal(r.reason.status, 429)
    assert.ok(Number.isFinite(r.reason.retryAfterSeconds))
  }
  const snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.heldOutputTokens, allowance)
  assert.equal(snap.remainingOutputTokens, 0)
})

test('per-user concurrency limit is distinct and retryable', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1', { maxConcurrent: 1 })
  const first = await adm.admit({ userId: 'u1', requestId: 'a', route: 'enhance', requestedOutputTokens: 10 })
  assert.equal(first.duplicate, false)
  await assert.rejects(
    adm.admit({ userId: 'u1', requestId: 'b', route: 'enhance', requestedOutputTokens: 10 }),
    (e) => e.kind === 'concurrency_limit' && e.status === 429 && e.retryAfterSeconds === 5
  )
})

test('global concurrency bound enforced across users', async (t) => {
  const { pool } = await freshDb(t)
  const adm = getAdmission(pool, mkConfig({ maxGlobalActive: 1 }))
  await adm.init()
  await provision(pool, 'u1')
  await provision(pool, 'u2')
  await adm.admit({ userId: 'u1', requestId: 'a', route: 'enhance', requestedOutputTokens: 10 })
  await assert.rejects(
    adm.admit({ userId: 'u2', requestId: 'b', route: 'enhance', requestedOutputTokens: 10 }),
    (e) => e.kind === 'concurrency_limit' && e.reason === 'global_capacity'
  )
})

test('duplicate admission returns the same reservation, not a new paid request', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1')
  const a = await adm.admit({ userId: 'u1', requestId: 'same', route: 'outline', requestedOutputTokens: 500 })
  const b = await adm.admit({ userId: 'u1', requestId: 'same', route: 'outline', requestedOutputTokens: 500 })
  assert.equal(a.reservationId, b.reservationId)
  assert.equal(b.duplicate, true)
  const n = await pool.query(`SELECT COUNT(*) AS c FROM public.ai_reservations`)
  assert.equal(Number(n.rows[0].c), 1)
})

test('duplicate admission after settlement replays stored response', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1')
  const a = await adm.admit({ userId: 'u1', requestId: 'same', route: 'enhance', requestedOutputTokens: 500 })
  await adm.settle({
    reservationId: a.reservationId,
    userId: 'u1',
    inputTokens: 50,
    outputTokens: 420,
    model: 'test-model',
    responsePayload: { result: 'improved prose' },
  })
  const b = await adm.admit({ userId: 'u1', requestId: 'same', route: 'enhance', requestedOutputTokens: 500 })
  assert.equal(b.duplicate, true)
  assert.equal(b.status, 'settled')
  assert.deepEqual(b.responsePayload, { result: 'improved prose' })
  const snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.settledOutputTokens, 420)
})

test('settlement is idempotent: one usage event, tokens counted once', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1')
  const a = await adm.admit({ userId: 'u1', requestId: 'r', route: 'enhance', requestedOutputTokens: 500 })
  const s1 = await adm.settle({
    reservationId: a.reservationId, userId: 'u1', inputTokens: 10, outputTokens: 300,
    responsePayload: { v: 1 },
  })
  assert.equal(s1.duplicate, false)
  const s2 = await adm.settle({
    reservationId: a.reservationId, userId: 'u1', inputTokens: 10, outputTokens: 300,
    responsePayload: { v: 2 },
  })
  assert.equal(s2.duplicate, true)
  assert.deepEqual(s2.responsePayload, { v: 1 }) // first write wins
  const ev = await pool.query(`SELECT COUNT(*) AS c FROM public.ai_usage_events`)
  assert.equal(Number(ev.rows[0].c), 1)
  const snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.settledOutputTokens, 300)
  assert.equal(snap.heldOutputTokens, 0)
})

test('settlement validates ownership', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'owner')
  await provision(pool, 'attacker')
  const a = await adm.admit({ userId: 'owner', requestId: 'r', route: 'enhance', requestedOutputTokens: 100 })
  await assert.rejects(
    adm.settle({ reservationId: a.reservationId, userId: 'attacker', inputTokens: 0, outputTokens: 999 }),
    (e) => e.kind === 'ownership' && e.status === 403
  )
  const ev = await pool.query(`SELECT COUNT(*) AS c FROM public.ai_usage_events`)
  assert.equal(Number(ev.rows[0].c), 0)
})

test('release frees the hold but keeps the idempotency key consumed', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1', { maxConcurrent: 1, allowance: 100 })
  const a = await adm.admit({ userId: 'u1', requestId: 'r', route: 'enhance', requestedOutputTokens: 90 })
  await adm.release({ reservationId: a.reservationId, userId: 'u1', reason: 'validation failed before inference' })
  const snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.heldOutputTokens, 0)
  assert.equal(snap.activeReservations, 0)
  // Re-admission with the SAME request id: key consumed, no new paid request.
  const b = await adm.admit({ userId: 'u1', requestId: 'r', route: 'enhance', requestedOutputTokens: 90 })
  assert.equal(b.duplicate, true)
  assert.equal(b.status, 'released')
  assert.equal(b.reservationId, a.reservationId)
  // A DIFFERENT request id can use the freed budget.
  const c = await adm.admit({ userId: 'u1', requestId: 'r2', route: 'enhance', requestedOutputTokens: 90 })
  assert.equal(c.duplicate, false)
})

test('uncertain-failure reconciliation holds quota; does not release', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1', { allowance: 200, maxConcurrent: 2 })
  const a = await adm.admit({ userId: 'u1', requestId: 'r', route: 'book', requestedOutputTokens: 150 })
  // Simulate TTL elapse: backdate creation beyond the TTL.
  await pool.query(
    `UPDATE public.ai_reservations SET created_at = NOW() - interval '2 hours' WHERE id = $1`,
    [a.reservationId]
  )
  const ids = await adm.reconcileUnknown()
  assert.deepEqual(ids, [a.reservationId])
  const snap = await adm.getUsageSnapshot('u1')
  // Still holds: unknown spend is not zero spend.
  assert.equal(snap.heldOutputTokens, 150)
  assert.equal(snap.activeReservations, 1)
  // Budget is not freed for new work while state is unknown.
  await assert.rejects(
    adm.admit({ userId: 'u1', requestId: 'other', route: 'book', requestedOutputTokens: 100 }),
    (e) => e.kind === 'quota_exceeded'
  )
  // Idempotent reconciliation: running again does not duplicate or change state.
  const again = await adm.reconcileUnknown()
  assert.deepEqual(again, [])
  const snap2 = await adm.getUsageSnapshot('u1')
  assert.equal(snap2.heldOutputTokens, 150)
})

test('operator resolves unknown reservation: spent and not-spent paths', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1', { allowance: 200, maxConcurrent: 2 })
  const a = await adm.admit({ userId: 'u1', requestId: 'r', route: 'book', requestedOutputTokens: 150 })
  await pool.query(
    `UPDATE public.ai_reservations SET created_at = NOW() - interval '2 hours' WHERE id = $1`,
    [a.reservationId]
  )
  await adm.reconcileUnknown()

  // not_spent: operator confirmed zero spend -> hold released, budget back.
  await adm.resolveUnknown({ reservationId: a.reservationId, userId: 'u1', outcome: 'not_spent' })
  let snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.heldOutputTokens, 0)
  assert.equal(snap.settledOutputTokens, 0)

  // spent path on a second reservation.
  const b = await adm.admit({ userId: 'u1', requestId: 'r2', route: 'book', requestedOutputTokens: 100 })
  await pool.query(
    `UPDATE public.ai_reservations SET created_at = NOW() - interval '2 hours' WHERE id = $1`,
    [b.reservationId]
  )
  await adm.reconcileUnknown()
  await adm.resolveUnknown({
    reservationId: b.reservationId, userId: 'u1', outcome: 'spent',
    inputTokens: 5, outputTokens: 80,
  })
  snap = await adm.getUsageSnapshot('u1')
  assert.equal(snap.settledOutputTokens, 80)
  assert.equal(snap.heldOutputTokens, 0)
  const ev = await pool.query(`SELECT COUNT(*) AS c FROM public.ai_usage_events`)
  assert.equal(Number(ev.rows[0].c), 1)
})

test('settle after duplicate concurrent settlement attempts yields one event (restart race)', async (t) => {
  const { pool, adm } = await freshDb(t)
  await provision(pool, 'u1')
  const a = await adm.admit({ userId: 'u1', requestId: 'r', route: 'enhance', requestedOutputTokens: 500 })
  const [s1, s2] = await Promise.allSettled([
    adm.settle({ reservationId: a.reservationId, userId: 'u1', inputTokens: 1, outputTokens: 100, responsePayload: { x: 1 } }),
    adm.settle({ reservationId: a.reservationId, userId: 'u1', inputTokens: 1, outputTokens: 100, responsePayload: { x: 1 } }),
  ])
  assert.equal(s1.status, 'fulfilled')
  assert.equal(s2.status, 'fulfilled')
  const ev = await pool.query(`SELECT output_tokens FROM public.ai_usage_events`)
  assert.equal(ev.rows.length, 1)
  assert.equal(Number(ev.rows[0].output_tokens), 100)
})

test('usage snapshot for unprovisioned user is null-entitlement, not an error', async (t) => {
  const { adm } = await freshDb(t)
  const snap = await adm.getUsageSnapshot('ghost')
  assert.equal(snap.entitlement, null)
  assert.equal(snap.remainingOutputTokens, null)
})
