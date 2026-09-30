import type { Pool } from 'pg'
import type { AiOverride } from './ai'
import { decryptSecret } from './crypto'
import { isPrivateAddress } from './ssrf-guard'

/**
 * Per-user AI provider settings (bring-your-own-key / bring-your-own-inference).
 *
 * provider = 'default'           → platform inference (LiteLLM gateway, env-configured)
 * provider = 'openai-compatible' → user-supplied OpenAI-compatible endpoint + key + model
 */

export interface UserAiSettings {
  provider: 'default' | 'openai-compatible'
  baseUrl: string | null
  model: string | null
  hasApiKey: boolean
  updatedAt: string | null
}

const ENSURE_TABLE_SQL = `
  CREATE TABLE IF NOT EXISTS public.user_ai_settings (
    user_id            VARCHAR(255) PRIMARY KEY,
    provider           VARCHAR(50)  NOT NULL DEFAULT 'default',
    base_url           TEXT,
    model              TEXT,
    api_key_encrypted  TEXT,
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW()
  )
`

let tableEnsured = false
export async function ensureAiSettingsTable(pool: Pool): Promise<void> {
  if (tableEnsured) return
  await pool.query(ENSURE_TABLE_SQL)
  tableEnsured = true
}

const PRIVATE_HOST_PATTERNS = [
  /^localhost$/i,
  /\.cluster\.local$/i,
  /\.internal$/i,
  /\.local$/i,
]

/**
 * Validates a user-supplied inference endpoint at save time.
 *
 * This is a fast pre-check only, NOT the SSRF enforcement point: hostname
 * strings say nothing about what DNS later resolves to (rebinding). Real
 * enforcement happens at connection time in lib/ssrf-guard.ts (see safeFetch,
 * used by lib/ai.ts for every user-override call): every resolved address is
 * range-checked as the socket connects, literal IPv4/IPv6 (including
 * IPv4-mapped ::ffff:0:0/96, unique-local fc00::/7, link-local fe80::/10)
 * hosts are blocked, embedded URL credentials are rejected, and redirects are
 * followed manually with per-hop re-validation.
 *
 * AI_ALLOW_PRIVATE_ENDPOINTS=true re-permits private endpoints (self-hosted
 * single-tenant deployments where internal inference is the point).
 */
export function validateInferenceUrl(raw: string): { ok: true; url: string } | { ok: false; error: string } {
  let parsed: URL
  try {
    parsed = new URL(raw)
  } catch {
    return { ok: false, error: 'Invalid URL' }
  }
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    return { ok: false, error: 'URL must be http(s)' }
  }
  if (parsed.username || parsed.password) {
    return { ok: false, error: 'Embedded credentials in the URL are not allowed' }
  }
  const allowPrivate = process.env.AI_ALLOW_PRIVATE_ENDPOINTS === 'true'
  // Literal IPs get the full range check (IPv4 + all IPv6 forms); hostnames
  // are only pattern-checked here and DNS-validated at connect time.
  const literalIp =
    /^\d{1,3}(\.\d{1,3}){3}$/.test(parsed.hostname) || parsed.hostname.includes(':')
  if (
    !allowPrivate &&
    (PRIVATE_HOST_PATTERNS.some((p) => p.test(parsed.hostname)) ||
      (literalIp && isPrivateAddress(parsed.hostname)))
  ) {
    return { ok: false, error: 'Private or cluster-internal endpoints are not allowed' }
  }
  return { ok: true, url: raw.replace(/\/+$/, '') }
}

interface AiSettingsRow {
  provider: string
  base_url: string | null
  model: string | null
  api_key_encrypted: string | null
  updated_at: string | null
}

async function loadRow(pool: Pool, userId: string): Promise<AiSettingsRow | null> {
  await ensureAiSettingsTable(pool)
  const result = await pool.query(
    `SELECT provider, base_url, model, api_key_encrypted, updated_at
       FROM public.user_ai_settings WHERE user_id = $1`,
    [userId]
  )
  return result.rows[0] ?? null
}

export async function getUserAiSettings(pool: Pool, userId: string): Promise<UserAiSettings> {
  const row = await loadRow(pool, userId)
  if (!row || row.provider !== 'openai-compatible') {
    return { provider: 'default', baseUrl: null, model: null, hasApiKey: false, updatedAt: row?.updated_at ?? null }
  }
  return {
    provider: 'openai-compatible',
    baseUrl: row.base_url,
    model: row.model,
    hasApiKey: Boolean(row.api_key_encrypted),
    updatedAt: row.updated_at,
  }
}

/**
 * Resolves the AI override for a user, decrypting the stored key.
 * Returns null when the user is on the platform default.
 *
 * FAIL-CLOSED for unguarded downstream consumers: /api/generate/book forwards
 * the override's base_url + key to the Rust book-generator, which dials the
 * endpoint itself (via the shared homelab-inference crate) with NO
 * connect-time SSRF guard. Until the generator enforces connection-time
 * address validation (see docs/integration-proposals/auth-byok-hardening.md),
 * overrides are suppressed and generation falls back to the platform gateway
 * for every consumer of this helper. Set BYOK_GENERATOR_GUARDED=true only
 * after the generator patch ships, restoring BYOK for outline/enhance/book.
 */
export async function getUserAiOverride(pool: Pool, userId: string): Promise<AiOverride | null> {
  if (process.env.BYOK_GENERATOR_GUARDED !== 'true') {
    console.warn(
      '[security] BYOK inference overrides suppressed (fail-closed): the book-generator path ' +
        'lacks connect-time endpoint enforcement. See docs/integration-proposals/auth-byok-hardening.md; ' +
        'set BYOK_GENERATOR_GUARDED=true once the generator guard ships.'
    )
    return null
  }
  let row: AiSettingsRow | null
  try {
    row = await loadRow(pool, userId)
  } catch (error) {
    console.warn('user_ai_settings lookup failed, using platform default:', error)
    return null
  }
  if (!row || row.provider !== 'openai-compatible' || !row.base_url || !row.model) {
    return null
  }
  const validation = validateInferenceUrl(row.base_url)
  if (!validation.ok) {
    console.warn(`Stored inference URL rejected for user ${userId}: ${validation.error}`)
    return null
  }
  let apiKey: string | undefined
  if (row.api_key_encrypted) {
    apiKey = decryptSecret(row.api_key_encrypted)
  }
  return { baseUrl: validation.url, model: row.model, apiKey }
}
