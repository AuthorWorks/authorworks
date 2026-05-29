import { NextRequest, NextResponse } from 'next/server'

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

/**
 * Resolves the Logto user info for a Bearer-authenticated request.
 * Returns `null` when the token is missing, malformed, or rejected by Logto.
 */
export async function resolveUser(request: NextRequest): Promise<LogtoUserInfo | null> {
  const authHeader = request.headers.get('authorization')
  if (!authHeader?.startsWith('Bearer ')) return null

  const token = authHeader.substring(7).trim()
  if (!token) return null

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

/** Convenience helper that returns just the Logto subject (user id). */
export async function getUserId(request: NextRequest): Promise<string | null> {
  const user = await resolveUser(request)
  return user?.sub ?? null
}

/** Standard 401 response used by API routes guarded by Logto. */
export function unauthorized(): NextResponse {
  return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
}
