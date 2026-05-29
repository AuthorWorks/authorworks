import { NextRequest, NextResponse } from 'next/server'
import { getLogtoEndpoint } from '@/app/lib/auth'

/**
 * POST /api/auth/logout
 *
 * Body: { refreshToken?: string }
 *
 * Performs server-side Logto session revocation when a refresh token is supplied.
 * Access tokens are bearer tokens with short expiry; clients still need to clear
 * local storage on their side. The Logto end-session URL must be hit from the
 * browser to clear the SSO cookie - that is the responsibility of the client.
 */
export async function POST(request: NextRequest) {
  let refreshToken: string | undefined
  try {
    const body = await request.json().catch(() => ({}))
    refreshToken = body?.refreshToken
  } catch {
    // ignore - body is optional
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

  return NextResponse.json({ success: true })
}
