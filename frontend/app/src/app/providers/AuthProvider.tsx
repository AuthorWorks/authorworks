'use client'

import { useRouter } from 'next/navigation'
import { createContext, ReactNode, useCallback, useContext, useEffect, useState } from 'react'

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

interface AuthContextType extends AuthState {
  login: () => void
  signUp: () => void
  logout: () => void
  handleCallback: (code: string, state: string) => Promise<void>
}

const AuthContext = createContext<AuthContextType | null>(null)

// Logto configuration
const LOGTO_ENDPOINT = process.env.NEXT_PUBLIC_LOGTO_ENDPOINT || 'http://localhost:3002'
const LOGTO_APP_ID = process.env.NEXT_PUBLIC_LOGTO_APP_ID || ''
const REDIRECT_URI = process.env.NEXT_PUBLIC_REDIRECT_URI || 'http://localhost:3001/callback'

// PKCE helpers
async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder()
  const data = encoder.encode(verifier)
  const digest = await crypto.subtle.digest('SHA-256', data)
  return base64UrlEncode(digest)
}

function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

function generateRandomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~'
  let result = ''
  const randomValues = new Uint8Array(length)
  crypto.getRandomValues(randomValues)
  for (let i = 0; i < length; i++) {
    result += chars[randomValues[i] % chars.length]
  }
  return result
}

export function AuthProvider({ children }: { children: ReactNode }) {
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
      const token = localStorage.getItem('accessToken')
      if (!token) {
        setAuthState(prev => ({ ...prev, isLoading: false }))
        return
      }

      // Add timeout to prevent stalling
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), 5000)

      const response = await fetch('/api/auth/me', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

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
      // On timeout or network error, clear potentially stale token
      console.error('Session check failed:', error)
      localStorage.removeItem('accessToken')
      setAuthState(prev => ({ ...prev, isLoading: false }))
    }
  }

  const initiateAuth = useCallback(async (isSignUp: boolean = false) => {
    // Clear any stale tokens before starting new auth flow
    localStorage.removeItem('accessToken')
    localStorage.removeItem('oauth_state')
    localStorage.removeItem('code_verifier')

    // Generate state for CSRF protection
    const state = generateRandomString(32)
    localStorage.setItem('oauth_state', state)

    // Generate PKCE code verifier (43-128 characters per RFC 7636)
    const codeVerifier = generateRandomString(64)
    localStorage.setItem('code_verifier', codeVerifier)

    // Generate S256 code challenge
    const codeChallenge = await generateCodeChallenge(codeVerifier)

    // Build authorization URL
    const params = new URLSearchParams({
      client_id: LOGTO_APP_ID,
      redirect_uri: REDIRECT_URI,
      response_type: 'code',
      scope: 'openid profile email offline_access',
      state,
      code_challenge: codeChallenge,
      code_challenge_method: 'S256',
    })

    // Add interaction mode for sign-up flow
    if (isSignUp) {
      params.set('first_screen', 'register')
    }

    window.location.href = `${LOGTO_ENDPOINT}/oidc/auth?${params.toString()}`
  }, [])

  const login = useCallback(() => {
    initiateAuth(false)
  }, [initiateAuth])

  const signUp = useCallback(() => {
    initiateAuth(true)
  }, [initiateAuth])

  const handleCallback = useCallback(async (code: string, state: string) => {
    // Verify state
    const savedState = localStorage.getItem('oauth_state')
    if (state !== savedState) {
      throw new Error('Invalid state parameter')
    }

    const codeVerifier = localStorage.getItem('code_verifier')
    if (!codeVerifier) {
      throw new Error('Missing code verifier')
    }

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
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || 'Token exchange failed')
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

    // Clear local auth state
    localStorage.removeItem('accessToken')
    localStorage.removeItem('oauth_state')
    localStorage.removeItem('code_verifier')

    setAuthState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      accessToken: null,
    })

    // Redirect to home page
    window.location.href = '/'
  }, [])

  return (
    <AuthContext.Provider
      value={{
        ...authState,
        login,
        signUp,
        logout,
        handleCallback,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
