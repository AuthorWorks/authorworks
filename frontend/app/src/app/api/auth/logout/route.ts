import { NextRequest, NextResponse } from 'next/server'
import { getLogtoEndpoint } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { SESSION_COOKIE, readSessionCookie, revokeSession } from '@/app/lib/session'

/**
 * POST /api/auth/logout
 *
 * Body: { refreshToken?: string } (optional; non-browser clients)
 *
 * Revokes the caller's server-side session: the stored refresh token is
 * revoked at Logto and the session row is marked revoked, so the HttpOnly
 * session cookie stops authenticating immediately. The cookie itself is
 * cleared in the response. The Logto end-session URL must still be hit from
 * the browser to clear the SSO cookie - that is the responsibility of the client.
 */
export async function POST(request: NextRequest) {
  let refreshToken: string | undefined
  try {
    const body = await request.json().catch(() => ({}))
    refreshToken = body?.refreshToken
  } catch {
    // ignore - body is optional
  }

  // Revoke the cookie-based session first; its stored refresh token is used
  // for the Logto revocation below when the body didn't supply one.
  const sessionId = readSessionCookie(request)
  if (sessionId) {
    try {
      const session = await revokeSession(getPool(), sessionId)
      if (session?.refreshToken) {
        refreshToken = refreshToken ?? session.refreshToken
      }
    } catch (error) {
      console.warn('Session revocation failed:', error)
    }
  }

  const clientId = process.env.LOGTO_CLIENT_ID || process.env.NEXT_PUBLIC_LOGTO_CLIENT_ID

  if (refreshToken && clientId) {
    try {
      await fetch(`${getLogtoEndpoint()}/oidc/token/revocation`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({
          token: refreshToken,
          token_type_hint: 'refresh_token',
          client_id: clientId,
        }).toString(),
      })
    } catch (error) {
      console.warn('Logto token revocation failed:', error)
    }
  }

  const response = NextResponse.json({ success: true })
  response.headers.append(
    'Set-Cookie',
    `${SESSION_COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0`
  )
  return response
}
