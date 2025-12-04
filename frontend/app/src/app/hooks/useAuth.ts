'use client'

import { useCallback, useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'

interface User {
  id: string
  name: string
  email: string
  avatar?: string
}

interface AuthState {
  user: User | null
  isAuthenticated: boolean
  isLoading: boolean
  accessToken: string | null
}

// Logto configuration
const LOGTO_ENDPOINT = process.env.NEXT_PUBLIC_LOGTO_ENDPOINT || 'http://localhost:3002'
const LOGTO_APP_ID = process.env.NEXT_PUBLIC_LOGTO_APP_ID || ''
const REDIRECT_URI = process.env.NEXT_PUBLIC_REDIRECT_URI || 'http://localhost:3001/callback'

export function useAuth() {
  const router = useRouter()
  const [authState, setAuthState] = useState<AuthState>({
    user: null,
    isAuthenticated: false,
    isLoading: true,
    accessToken: null,
  })

  // Check for existing session on mount
  useEffect(() => {
    checkSession()
  }, [])

  const checkSession = async () => {
    try {
      // Check if we have a token stored
      const token = localStorage.getItem('accessToken')
      if (!token) {
        setAuthState(prev => ({ ...prev, isLoading: false }))
        return
      }

      // Validate token with backend
      const response = await fetch('/api/auth/me', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      })

      if (response.ok) {
        const user = await response.json()
        setAuthState({
          user,
          isAuthenticated: true,
          isLoading: false,
          accessToken: token,
        })
      } else {
        // Token invalid, clear it
        localStorage.removeItem('accessToken')
        setAuthState(prev => ({ ...prev, isLoading: false }))
      }
    } catch (error) {
      console.error('Session check failed:', error)
      setAuthState(prev => ({ ...prev, isLoading: false }))
    }
  }

  const login = useCallback(() => {
    // Generate state for CSRF protection
    const state = generateRandomString(32)
    localStorage.setItem('oauth_state', state)

    // Generate PKCE code verifier and challenge
    const codeVerifier = generateRandomString(64)
    localStorage.setItem('code_verifier', codeVerifier)

    // Build authorization URL
    const params = new URLSearchParams({
      client_id: LOGTO_APP_ID,
      redirect_uri: REDIRECT_URI,
      response_type: 'code',
      scope: 'openid profile email',
      state,
      code_challenge: codeVerifier, // In production, this should be SHA256 hashed
      code_challenge_method: 'plain', // Should be 'S256' in production
    })

    window.location.href = `${LOGTO_ENDPOINT}/oidc/auth?${params.toString()}`
  }, [])

  const handleCallback = useCallback(async (code: string, state: string) => {
    // Verify state
    const savedState = localStorage.getItem('oauth_state')
    if (state !== savedState) {
      throw new Error('Invalid state parameter')
    }

    const codeVerifier = localStorage.getItem('code_verifier')

    // Exchange code for tokens
    const response = await fetch('/api/auth/callback', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        code,
        codeVerifier,
        redirectUri: REDIRECT_URI,
      }),
    })

    if (!response.ok) {
      throw new Error('Token exchange failed')
    }

    const { accessToken, user } = await response.json()

    // Store token
    localStorage.setItem('accessToken', accessToken)
    localStorage.removeItem('oauth_state')
    localStorage.removeItem('code_verifier')

    setAuthState({
      user,
      isAuthenticated: true,
      isLoading: false,
      accessToken,
    })

    router.push('/dashboard')
  }, [router])

  const logout = useCallback(async () => {
    try {
      // Notify backend
      const token = localStorage.getItem('accessToken')
      if (token) {
        await fetch('/api/auth/logout', {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${token}`,
          },
        })
      }
    } catch (error) {
      console.error('Logout error:', error)
    }

    // Clear local state
    localStorage.removeItem('accessToken')
    setAuthState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      accessToken: null,
    })

    // Redirect to Logto logout
    const params = new URLSearchParams({
      client_id: LOGTO_APP_ID,
      post_logout_redirect_uri: window.location.origin,
    })
    window.location.href = `${LOGTO_ENDPOINT}/oidc/session/end?${params.toString()}`
  }, [])

  return {
    ...authState,
    login,
    logout,
    handleCallback,
  }
}

function generateRandomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
  let result = ''
  const randomValues = new Uint8Array(length)
  crypto.getRandomValues(randomValues)
  for (let i = 0; i < length; i++) {
    result += chars[randomValues[i] % chars.length]
  }
  return result
}

