import { NextRequest, NextResponse } from 'next/server'
import { getLogtoEndpoint, toClientUser, type LogtoUserInfo } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { SESSION_COOKIE, createSession, sessionCookieAttributes } from '@/app/lib/session'

/**
 * POST /api/auth/callback
 *
 * Server-side leg of the Logto PKCE flow:
 *  1. Exchange the auth code + verifier for tokens at Logto.
 *  2. Resolve the user via /oidc/me so the client gets a ready-to-use session.
 *  3. Best-effort sync of the resulting user to a downstream user-service when configured.
 *  4. Persist a server-side session (access + refresh tokens encrypted at
 *     rest) and set a single HttpOnly, Secure, SameSite=Strict cookie.
 *
 * Tokens are NEVER returned to the browser: no bearer token lands in
 * localStorage, and the refresh token is no longer discarded — the server
 * keeps it to renew the session transparently.
 *
 * Returns { user } — the client session is the cookie.
 */
export async function POST(request: NextRequest) {
  let body: { code?: string; codeVerifier?: string; redirectUri?: string }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const { code, codeVerifier, redirectUri } = body
  if (!code || !codeVerifier || !redirectUri) {
    return NextResponse.json({ error: 'Missing required parameters' }, { status: 400 })
  }

  const clientId =
    process.env.LOGTO_APP_ID ||
    process.env.LOGTO_CLIENT_ID ||
    process.env.NEXT_PUBLIC_LOGTO_APP_ID ||
    process.env.NEXT_PUBLIC_LOGTO_CLIENT_ID
  if (!clientId) {
    return NextResponse.json({ error: 'Logto app id is not configured' }, { status: 500 })
  }

  try {
    const tokenResponse = await fetch(`${getLogtoEndpoint()}/oidc/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: redirectUri,
        client_id: clientId,
        code_verifier: codeVerifier,
      }).toString(),
    })

    if (!tokenResponse.ok) {
      const errorText = await tokenResponse.text()
      return NextResponse.json(
        { error: 'Token exchange failed', details: errorText },
        { status: tokenResponse.status }
      )
    }

    const tokens = await tokenResponse.json()
    if (!tokens.access_token) {
      return NextResponse.json({ error: 'Token exchange returned no access token' }, { status: 502 })
    }

    // Resolve the user so the client can hydrate its session in one round trip.
    // A session is only minted when the user is positively resolved (fail closed).
    let user = null
    try {
      const userResponse = await fetch(`${getLogtoEndpoint()}/oidc/me`, {
        headers: { Authorization: `Bearer ${tokens.access_token}` },
        cache: 'no-store',
      })
      if (userResponse.ok) {
        user = toClientUser((await userResponse.json()) as LogtoUserInfo)
      }
    } catch (error) {
      console.warn('Userinfo fetch failed during callback:', error)
    }
    if (!user) {
      return NextResponse.json(
        { error: 'Authentication callback failed to resolve user' },
        { status: 502 }
      )
    }

    // Optional downstream user-sync. Failure here is non-blocking: the user is still
    // authenticated and the JWT is the source of truth for the rest of the app.
    const userServiceUrl = process.env.USER_SERVICE_URL
    if (userServiceUrl) {
      try {
        await fetch(`${userServiceUrl}/auth/sync`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${tokens.access_token}`,
          },
          body: JSON.stringify({ source: 'logto-callback' }),
        })
      } catch (error) {
        console.warn('User sync skipped:', error)
      }
    }

    // Mint the server-side session: tokens stay server-side (encrypted at
    // rest), the browser only holds the signed opaque session cookie.
    let cookieValue: string
    try {
      cookieValue = await createSession(
        getPool(),
        user.id,
        tokens.access_token,
        tokens.refresh_token ?? null
      )
    } catch (error) {
      console.error('Session creation failed:', error)
      return NextResponse.json({ error: 'Failed to establish session' }, { status: 500 })
    }

    const response = NextResponse.json({ user })
    response.headers.append(
      'Set-Cookie',
      `${SESSION_COOKIE}=${cookieValue}; ${sessionCookieAttributes()}`
    )
    return response
  } catch (error) {
    console.error('Logto callback failed:', error)
    return NextResponse.json({ error: 'Authentication callback failed' }, { status: 500 })
  }
}
