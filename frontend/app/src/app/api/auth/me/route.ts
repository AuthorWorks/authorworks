import { NextRequest, NextResponse } from 'next/server'

export async function GET(request: NextRequest) {
  // Read env var inside handler to avoid build-time caching
  const LOGTO_ENDPOINT = process.env.LOGTO_ENDPOINT || 'http://localhost:3002'

  console.log('Auth /me check - LOGTO_ENDPOINT:', LOGTO_ENDPOINT)

  try {
    const authHeader = request.headers.get('authorization')
    if (!authHeader?.startsWith('Bearer ')) {
      return NextResponse.json(
        { error: 'Missing authorization header' },
        { status: 401 }
      )
    }

    const token = authHeader.substring(7)

    // Validate token with Logto
    const userInfoResponse = await fetch(`${LOGTO_ENDPOINT}/oidc/me`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    })

    if (!userInfoResponse.ok) {
      return NextResponse.json(
        { error: 'Invalid token' },
        { status: 401 }
      )
    }

    const userInfo = await userInfoResponse.json()

    return NextResponse.json({
      id: userInfo.sub,
      name: userInfo.name || userInfo.username || 'User',
      email: userInfo.email,
      avatar: userInfo.picture,
    })
  } catch (error) {
    console.error('Auth check error:', error)
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    )
  }
}

