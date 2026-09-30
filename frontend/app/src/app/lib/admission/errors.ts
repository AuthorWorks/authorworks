/**
 * Admission error taxonomy. Each kind maps to a distinct HTTP contract so
 * callers (and the UI) can distinguish quota, entitlement, concurrency and
 * fail-closed-configuration responses.
 */

export type AdmissionErrorKind =
  | 'invalid_request' // 400 — request exceeds per-request limits; not retryable as-is
  | 'entitlement_missing' // 402 — user has no plan / subscription inactive
  | 'quota_exceeded' // 429 — retryable; Retry-After points at period reset
  | 'concurrency_limit' // 429 — retryable; Retry-After points at a short backoff
  | 'admission_disabled' // 503 — platform funding/billing not configured; fail closed
  | 'ownership' // 403 — reservation belongs to another user
  | 'internal' // 500

const STATUS_BY_KIND: Record<AdmissionErrorKind, number> = {
  invalid_request: 400,
  entitlement_missing: 402,
  quota_exceeded: 429,
  concurrency_limit: 429,
  admission_disabled: 503,
  ownership: 403,
  internal: 500,
}

export class AdmissionError extends Error {
  readonly kind: AdmissionErrorKind
  readonly status: number
  /** Seconds; present only for retryable limits (quota, concurrency). */
  readonly retryAfterSeconds?: number
  /** Stable machine-readable reason for the response body. */
  readonly reason: string

  constructor(
    kind: AdmissionErrorKind,
    message: string,
    opts: { retryAfterSeconds?: number; reason?: string } = {}
  ) {
    super(message)
    this.name = 'AdmissionError'
    this.kind = kind
    this.status = STATUS_BY_KIND[kind]
    this.reason = opts.reason ?? kind
    this.retryAfterSeconds = opts.retryAfterSeconds
  }

  toJSON() {
    return {
      error: 'admission_denied',
      kind: this.kind,
      reason: this.reason,
      message: this.message,
      ...(this.retryAfterSeconds !== undefined && { retry_after: this.retryAfterSeconds }),
    }
  }
}

/** Helper for API routes: converts any thrown AdmissionError to a response. */
export function admissionErrorResponse(error: AdmissionError): {
  status: number
  body: ReturnType<AdmissionError['toJSON']>
  headers: Record<string, string>
} {
  const headers: Record<string, string> = {}
  if (error.retryAfterSeconds !== undefined) {
    // Retry-After belongs exclusively to retryable limits.
    headers['Retry-After'] = String(Math.max(0, Math.ceil(error.retryAfterSeconds)))
  }
  return { status: error.status, body: error.toJSON(), headers }
}
