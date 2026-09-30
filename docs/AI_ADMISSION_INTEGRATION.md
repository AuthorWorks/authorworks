# AI Admission & Usage Accounting — Shared-Route Integration Contract

Status: implemented and tested in isolation (see `tests/admission.test.mjs`,
16/16 passing against a throwaway Postgres). The shared generation routes are
NOT modified in this branch — this document is the contract for the owning
lanes to integrate.

## Module API

`@/app/lib/admission` (re-exports `getAdmission`, `Admission`,
`AdmissionError`, `admissionErrorResponse`):

- `await admission.init()` — idempotent, applies the additive schema (also
  available as `migrations/2026-09-30-ai-admission.sql` for out-of-band apply).
- `admit({ userId, requestId, route, requestedOutputTokens })` →
  `{ reservationId, status, duplicate, responsePayload?, reservedOutputTokens }`
  or throws `AdmissionError`.
- `settle({ reservationId, userId, inputTokens, outputTokens, model?, responsePayload? })` →
  `{ reservationId, duplicate, responsePayload? }` (idempotent, exactly-once
  usage event, ownership-validated).
- `release({ reservationId, userId, reason? })` — pre-inference failure only.
- `reconcileUnknown({ olderThanSeconds? })` — TTL sweep for
  restart/periodic job; NEVER releases quota (unknown spend ≠ zero spend).
- `resolveUnknown({ reservationId, userId, outcome: 'spent'|'not_spent', … })` —
  operator-only resolution after external confirmation.
- `getUsageSnapshot(userId)` — for `/api/subscription` and settings UI.

## Error contract (distinct responses; Retry-After only on retryable limits)

| kind                 | status | Retry-After | meaning                                   |
|----------------------|--------|-------------|-------------------------------------------|
| `invalid_request`    | 400    | —           | exceeds per-request token cap / bad input |
| `entitlement_missing`| 402    | —           | no plan / plan paused or canceled         |
| `quota_exceeded`     | 429    | ✔           | monthly output-token allowance exhausted  |
| `concurrency_limit`  | 429    | ✔           | per-user in-flight cap or global capacity |
| `admission_disabled` | 503    | —           | platform funding not configured (fail closed) |
| `ownership`          | 403    | —           | reservation belongs to another user      |

Use `admissionErrorResponse(err)` to build `{ status, body, headers }`.

Read/edit/export routes must NOT call admission — entitlement state only
gates paid generation.

## Route patch (enhance / outline)

Manuscript lane owns book-start/status/sync; the same pattern applies there.

```ts
import { getAdmission, AdmissionError, admissionErrorResponse } from '@/app/lib/admission'
import { NextResponse } from 'next/server'

// 1. Require a client idempotency key.
const requestId = request.headers.get('x-request-id')
if (!requestId) return NextResponse.json({ error: 'x-request-id header required' }, { status: 400 })

// 2. Admission BEFORE any inference call, after request validation.
const admission = getAdmission(getPool())
let admit
try {
  await admission.init() // once at module scope is better
  admit = await admission.admit({
    userId,
    requestId,
    route: 'enhance',                       // 'outline' | 'book' on those routes
    requestedOutputTokens: body.max_tokens ?? 4000,
  })
} catch (error) {
  if (error instanceof AdmissionError) {
    const { status, body, headers } = admissionErrorResponse(error)
    return NextResponse.json(body, { status, headers })
  }
  throw error
}

// 3. Duplicate replay: return the SAME reservation/job; never re-run the paid request.
if (admit.duplicate) {
  if (admit.status === 'settled') {
    return NextResponse.json(admit.responsePayload)  // stored settlement payload
  }
  if (admit.status === 'released') {
    return NextResponse.json(
      { error: 'This request was already processed and cancelled', reservation_id: admit.reservationId },
      { status: 409 })
  }
  return NextResponse.json(                       // still in flight (reserved/unknown)
    { status: 'in_progress', reservation_id: admit.reservationId }, { status: 202 })
}

// 4. Pre-inference failure path (validation etc.) → release (frees the hold):
//    await admission.release({ reservationId: admit.reservationId, userId, reason: '...' })

// 5. After inference, settle with REAL token counts and the response payload:
//    const s = await admission.settle({ reservationId: admit.reservationId, userId,
//      inputTokens: completion.inputTokens, outputTokens: completion.outputTokens,
//      model: completion.model, responsePayload: <route response object> })
```

## Deployment / env

- `AI_ADMISSION_ENABLED=true` enables admission; default OFF (fail closed).
- `AI_ADMISSION_MAX_GLOBAL_ACTIVE` (default 3), `AI_ADMISSION_RESERVATION_TTL_S`
  (default 900), `AI_ADMISSION_RETRY_AFTER_S` (default 5).
- No prices, allowances, plan tiers, BYOK policy or billing authorization are
  invented here. `ai_entitlements` rows are provisioned by the billing lane;
  until they exist, users get `entitlement_missing` (402) with an actionable
  message; with the flag off, everyone gets `admission_disabled` (503).

## Remaining dependencies (not owned by this lane)

- Billing lane: entitlement provisioning, period reset, webhook → entitlement
  transitions (must be monotonic-aware; dedup alone does not prevent regression).
- Ops: schedule `reconcileUnknown` (startup + periodic) and an operator runbook
  for `resolveUnknown`; set `AI_ADMISSION_*` env in compose/k8s.
- Route owners: apply the patch above to `/api/generate/{enhance,outline,book}`,
  and surface usage from `getUsageSnapshot` in `/api/subscription`.
