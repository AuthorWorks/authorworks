import { NextRequest, NextResponse } from 'next/server'
import { getLogtoEndpoint, toClientUser, type LogtoUserInfo } from '@/app/lib/auth'

/**
 * POST /api/auth/callback
 *
 * Server-side leg of the Logto PKCE flow:
 *  1. Exchange the auth code + verifier for tokens at Logto.
 *  2. Resolve the user via /oidc/me so the client gets a ready-to-use session.
 *  3. Best-effort sync of the resulting user to a downstream user-service when configured.
 *
 * Returns { accessToken, refreshToken, expiresIn, user } — the contract the
 * client AuthProvider consumes.
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

    return NextResponse.json({
      accessToken: tokens.access_token,
      refreshToken: tokens.refresh_token ?? null,
      expiresIn: tokens.expires_in ?? null,
      user,
    })
  } catch (error) {
    console.error('Logto callback failed:', error)
    return NextResponse.json({ error: 'Authentication callback failed' }, { status: 500 })
  }
}
