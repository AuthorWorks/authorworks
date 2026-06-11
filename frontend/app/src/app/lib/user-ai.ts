import type { Pool } from 'pg'
import type { AiOverride } from './ai'
import { decryptSecret } from './crypto'

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
  /^127\./,
  /^0\.0\.0\.0$/,
  /^10\./,
  /^192\.168\./,
  /^172\.(1[6-9]|2\d|3[01])\./,
  /^169\.254\./,
  /\.cluster\.local$/i,
  /^\[?::1\]?$/,
  /^\[?fd[0-9a-f]{2}:/i,
]

/**
 * Validates a user-supplied inference endpoint. Blocks cluster-internal and
 * private addresses unless AI_ALLOW_PRIVATE_ENDPOINTS=true (for self-hosted
 * single-tenant deployments where "internal inference" is the point).
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
  const allowPrivate = process.env.AI_ALLOW_PRIVATE_ENDPOINTS === 'true'
  if (!allowPrivate && PRIVATE_HOST_PATTERNS.some((p) => p.test(parsed.hostname))) {
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
 */
export async function getUserAiOverride(pool: Pool, userId: string): Promise<AiOverride | null> {
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
