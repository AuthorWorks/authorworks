import { NextRequest, NextResponse } from 'next/server'
import { getPool } from './db'
import { loadSession, readSessionCookie, updateSessionTokens } from './session'

const DEFAULT_LOGTO_ENDPOINT = 'http://logto.security.svc.cluster.local:3001'

export function getLogtoEndpoint(): string {
  return process.env.LOGTO_ENDPOINT || DEFAULT_LOGTO_ENDPOINT
}

export interface LogtoUserInfo {
  sub: string
  email?: string
  name?: string
  username?: string
  picture?: string
}

/** User shape consumed by the client AuthProvider. */
export interface ClientUser {
  id: string
  name: string
  email: string
  avatar?: string
}

export function toClientUser(info: LogtoUserInfo): ClientUser {
  return {
    id: info.sub,
    name: info.name || info.username || info.email || 'Author',
    email: info.email || '',
    avatar: info.picture,
  }
}

/**
 * Public OIDC config served to the browser at runtime. Reads plain (non
 * NEXT_PUBLIC) env vars first so values can be set per-deployment without
 * rebuilding the image; NEXT_PUBLIC_* build-time values remain the fallback
 * for local dev.
 */
export function getPublicAuthConfig() {
  return {
    endpoint:
      process.env.LOGTO_PUBLIC_ENDPOINT ||
      process.env.NEXT_PUBLIC_LOGTO_ENDPOINT ||
      'http://localhost:3002',
    appId:
      process.env.LOGTO_APP_ID ||
      process.env.NEXT_PUBLIC_LOGTO_APP_ID ||
      '',
    redirectUri:
      process.env.LOGTO_REDIRECT_URI ||
      process.env.NEXT_PUBLIC_REDIRECT_URI ||
      'http://localhost:3001/callback',
  }
}

/**
 * CSRF defense for cookie-authenticated requests (ambient credential):
 * non-GET requests must carry an Origin or Referer header whose host matches
 * the request's host (x-forwarded-host when behind a proxy). Fail-closed when
 * neither header is present. Bearer-header requests are exempt.
 */
export function assertSameOrigin(request: NextRequest): boolean {
  if (request.method === 'GET' || request.method === 'HEAD' || request.method === 'OPTIONS') {
    return true
  }
  const requestHost =
    request.headers.get('x-forwarded-host')?.split(',')[0].trim() || request.headers.get('host')
  const originHeader = request.headers.get('origin') || request.headers.get('referer')
  if (!requestHost || !originHeader) return false
  try {
    const origin = new URL(originHeader)
    return origin.host === requestHost
  } catch {
    return false
  }
}

async function validateAtLogto(token: string): Promise<LogtoUserInfo | null> {
  try {
    const response = await fetch(`${getLogtoEndpoint()}/oidc/me`, {
      headers: { Authorization: `Bearer ${token}` },
      cache: 'no-store',
    })
    if (!response.ok) return null
    return (await response.json()) as LogtoUserInfo
  } catch {
    return null
  }
}

/**
 * Exchanges the stored refresh token at Logto for a fresh token pair,
 * rotating the encrypted values in the session store.
 */
async function refreshLogtoTokens(sessionId: string, refreshToken: string): Promise<string | null> {
  const clientId =
    process.env.LOGTO_APP_ID ||
    process.env.LOGTO_CLIENT_ID ||
    process.env.NEXT_PUBLIC_LOGTO_APP_ID ||
    process.env.NEXT_PUBLIC_LOGTO_CLIENT_ID
  if (!clientId) return null
  try {
    const response = await fetch(`${getLogtoEndpoint()}/oidc/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        refresh_token: refreshToken,
        client_id: clientId,
      }).toString(),
    })
    if (!response.ok) return null
    const tokens = await response.json()
    if (!tokens.access_token) return null
    await updateSessionTokens(getPool(), sessionId, tokens.access_token, tokens.refresh_token ?? null)
    return tokens.access_token as string
  } catch (error) {
    console.warn('Logto refresh failed:', error)
    return null
  }
}

/**
 * Resolves the Logto user for a request. Two credential paths:
 *
 *  1. Bearer header (non-browser clients / transition period) - validated
 *     directly at Logto. Unchanged behavior for API clients.
 *  2. Signed HttpOnly session cookie (browser) - CSRF origin check on
 *     state-changing methods, token validated at Logto, transparent refresh
 *     via the server-stored refresh token when the access token expired.
 *
 * Returns `null` when no valid credential is present.
 */
export async function resolveUser(request: NextRequest): Promise<LogtoUserInfo | null> {
  const authHeader = request.headers.get('authorization')

  if (authHeader?.startsWith('Bearer ')) {
    const token = authHeader.substring(7).trim()
    if (!token) return null
    return validateAtLogto(token)
  }

  const sessionId = readSessionCookie(request)
  if (!sessionId) return null

  if (!assertSameOrigin(request)) return null

  const session = await loadSession(getPool(), sessionId)
  if (!session) return null

  if (session.accessToken) {
    const user = await validateAtLogto(session.accessToken)
    if (user) return user
  }

  if (session.refreshToken) {
    const refreshed = await refreshLogtoTokens(sessionId, session.refreshToken)
    if (refreshed) {
      return validateAtLogto(refreshed)
    }
  }

  return null
}

/** Convenience helper that returns just the Logto subject (user id). */
export async function getUserId(request: NextRequest): Promise<string | null> {
  const user = await resolveUser(request)
  return user?.sub ?? null
}

/** Standard 401 response used by API routes guarded by Logto. */
export function unauthorized(): NextResponse {
  return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
}
