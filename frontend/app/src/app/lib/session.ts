import { createHmac, randomBytes, timingSafeEqual } from 'crypto'
import type { NextRequest } from 'next/server'
import type { Pool } from 'pg'
import { decryptSecret, encryptSecret } from './crypto'

/**
 * Server-side session store backing HttpOnly cookies.
 *
 * Design (auth audit remediation):
 *  - The browser never receives the Logto access or refresh token. The
 *    callback route stores both server-side (refresh token AES-256-GCM
 *    encrypted via the existing crypto helper) and hands the browser a single
 *    opaque, HMAC-signed session id in an HttpOnly, Secure, SameSite=Strict
 *    cookie. No bearer token is persisted to localStorage.
 *  - The refresh token is used only by the server to renew the session when
 *    the access token expires (rotating the stored pair), fixing the
 *    "refresh token returned but discarded" finding.
 *  - CSRF defense: SameSite=Strict plus an Origin/Referer same-host check on
 *    every non-GET cookie-authenticated request (see assertSameOrigin in
 *    lib/auth.ts). Bearer-header requests are exempt (not ambient-credential
 *    CSRF-able) so non-browser clients keep working.
 *  - Logto remains the identity provider: PKCE flow, token validation and
 *    revocation all still go through Logto endpoints.
 */

export const SESSION_COOKIE = 'aw_session'
const SESSION_TTL_SECONDS = 30 * 24 * 60 * 60 // 30 days
const SESSION_TTL_MS = SESSION_TTL_SECONDS * 1000

export interface SessionRow {
  sessionId: string
  userId: string
  accessToken: string
  refreshToken: string | null
}

const ENSURE_TABLE_SQL = `
  CREATE TABLE IF NOT EXISTS public.auth_sessions (
    session_id               VARCHAR(64) PRIMARY KEY,
    user_id                  VARCHAR(255) NOT NULL,
    access_token_encrypted   TEXT,
    refresh_token_encrypted  TEXT,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at               TIMESTAMPTZ NOT NULL,
    revoked_at               TIMESTAMPTZ
  )
`

let tableEnsured = false
async function ensureSessionsTable(pool: Pool): Promise<void> {
  if (tableEnsured) return
  await pool.query(ENSURE_TABLE_SQL)
  tableEnsured = true
}

function sessionSecret(): Buffer {
  // Dedicated secret preferred; the AI-key secret is an acceptable fallback so
  // deployments without a new env var still get signed (not guessable) ids.
  const secret = process.env.AUTH_SESSION_SECRET || process.env.AI_KEY_ENCRYPTION_SECRET
  if (!secret || secret.length < 16) {
    throw new Error('AUTH_SESSION_SECRET (or AI_KEY_ENCRYPTION_SECRET) is not configured (min 16 chars)')
  }
  return createHmac('sha256', secret).update('aw-session-signing-v1').digest()
}

/** Cookie value format: `<base64url session id>.<base64url HMAC>` */
export function signSessionId(sessionId: string): string {
  const mac = createHmac('sha256', sessionSecret()).update(sessionId).digest('base64url')
  return `${sessionId}.${mac}`
}

/** Verifies the cookie signature and returns the bare session id. */
export function verifySessionCookie(value: string | undefined): string | null {
  if (!value) return null
  const dot = value.lastIndexOf('.')
  if (dot <= 0) return null
  const sessionId = value.slice(0, dot)
  const mac = value.slice(dot + 1)
  const expected = createHmac('sha256', sessionSecret()).update(sessionId).digest('base64url')
  const a = Buffer.from(mac)
  const b = Buffer.from(expected)
  if (a.length !== b.length || !timingSafeEqual(a, b)) return null
  return sessionId
}

/**
 * Persists a new session for the authenticated user and returns the cookie
 * value to set. Tokens are encrypted at rest.
 */
export async function createSession(
  pool: Pool,
  userId: string,
  accessToken: string,
  refreshToken: string | null
): Promise<string> {
  await ensureSessionsTable(pool)
  const sessionId = randomBytes(32).toString('base64url')
  await pool.query(
    `INSERT INTO public.auth_sessions
       (session_id, user_id, access_token_encrypted, refresh_token_encrypted, expires_at)
     VALUES ($1, $2, $3, $4, NOW() + INTERVAL '30 days')`,
    [
      sessionId,
      userId,
      encryptSecret(accessToken),
      refreshToken ? encryptSecret(refreshToken) : null,
    ]
  )
  return signSessionId(sessionId)
}

/** Cookie serialization attributes shared by set and clear operations. */
export function sessionCookieAttributes(): string {
  return `Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=${SESSION_TTL_SECONDS}`
}

/** Loads a live (non-revoked, non-expired) session; null otherwise. */
export async function loadSession(
  pool: Pool,
  sessionId: string
): Promise<SessionRow | null> {
  await ensureSessionsTable(pool)
  const result = await pool.query(
    `SELECT user_id, access_token_encrypted, refresh_token_encrypted
       FROM public.auth_sessions
      WHERE session_id = $1 AND revoked_at IS NULL AND expires_at > NOW()`,
    [sessionId]
  )
  const row = result.rows[0]
  if (!row) return null
  try {
    return {
      sessionId,
      userId: row.user_id,
      accessToken: row.access_token_encrypted ? decryptSecret(row.access_token_encrypted) : '',
      refreshToken: row.refresh_token_encrypted ? decryptSecret(row.refresh_token_encrypted) : null,
    }
  } catch (error) {
    console.warn('auth_sessions decryption failed (rotated secret?):', error)
    return null
  }
}

/** Rotates the stored token pair after a Logto refresh-token grant. */
export async function updateSessionTokens(
  pool: Pool,
  sessionId: string,
  accessToken: string,
  refreshToken: string | null
): Promise<void> {
  await pool.query(
    `UPDATE public.auth_sessions
        SET access_token_encrypted = $2,
            refresh_token_encrypted = COALESCE($3, refresh_token_encrypted),
            last_used_at = NOW()
      WHERE session_id = $1`,
    [sessionId, encryptSecret(accessToken), refreshToken ? encryptSecret(refreshToken) : null]
  )
}

/** Revokes a session and best-effort deletes it. Idempotent. */
export async function revokeSession(pool: Pool, sessionId: string): Promise<SessionRow | null> {
  await ensureSessionsTable(pool)
  const existing = await loadSession(pool, sessionId)
  await pool.query(
    `UPDATE public.auth_sessions SET revoked_at = NOW() WHERE session_id = $1`,
    [sessionId]
  )
  return existing
}

/** Reads + verifies the session cookie on an incoming request. */
export function readSessionCookie(request: NextRequest): string | null {
  return verifySessionCookie(request.cookies.get(SESSION_COOKIE)?.value)
}
