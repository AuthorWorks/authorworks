import { NextRequest, NextResponse } from 'next/server'

const LOGTO_ENDPOINT = process.env.LOGTO_ENDPOINT || 'http://localhost:3002'
const LOGTO_APP_ID = process.env.LOGTO_APP_ID || ''
const LOGTO_APP_SECRET = process.env.LOGTO_APP_SECRET || ''

export async function POST(request: NextRequest) {
  try {
    const body = await request.json()
    const { code, codeVerifier, redirectUri } = body

    if (!code || !redirectUri) {
      return NextResponse.json(
        { error: 'Missing required parameters' },
        { status: 400 }
      )
    }

    // Exchange code for tokens with Logto
    const tokenResponse = await fetch(`${LOGTO_ENDPOINT}/oidc/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        Authorization: `Basic ${Buffer.from(`${LOGTO_APP_ID}:${LOGTO_APP_SECRET}`).toString('base64')}`,
      },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: redirectUri,
        code_verifier: codeVerifier || '',
      }),
    })

    if (!tokenResponse.ok) {
      const error = await tokenResponse.text()
      console.error('Token exchange failed:', error)
      return NextResponse.json(
        { error: 'Token exchange failed' },
        { status: 401 }
      )
    }

    const tokens = await tokenResponse.json()
    const { access_token, id_token } = tokens

    // Get user info from Logto
    const userInfoResponse = await fetch(`${LOGTO_ENDPOINT}/oidc/me`, {
      headers: {
        Authorization: `Bearer ${access_token}`,
      },
    })

    if (!userInfoResponse.ok) {
      return NextResponse.json(
        { error: 'Failed to get user info' },
        { status: 401 }
      )
    }

    const userInfo = await userInfoResponse.json()

    // Sync user to our database via User Service
    const userServiceUrl = process.env.USER_SERVICE_URL || 'http://localhost:8080'
    const syncResponse = await fetch(`${userServiceUrl}/api/users/sync`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${access_token}`,
      },
      body: JSON.stringify({
        logto_id: userInfo.sub,
        email: userInfo.email,
        name: userInfo.name || userInfo.username,
        avatar: userInfo.picture,
      }),
    })

    let userId = userInfo.sub
    if (syncResponse.ok) {
      const syncedUser = await syncResponse.json()
      userId = syncedUser.id
    }

    // Return user info and token to client
    return NextResponse.json({
      accessToken: access_token,
      user: {
        id: userId,
        name: userInfo.name || userInfo.username || 'User',
        email: userInfo.email,
        avatar: userInfo.picture,
      },
    })
  } catch (error) {
    console.error('Auth callback error:', error)
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    )
  }
}

