'use client';

import { useRouter } from 'next/navigation';
import { createContext, ReactNode, useCallback, useContext, useEffect, useState } from 'react';

/**
 * Browser session state.
 *
 * Security model (auth audit remediation): the browser holds NO tokens. The
 * session is a single HttpOnly, Secure, SameSite=Strict cookie minted by
 * /api/auth/callback; authenticated same-origin fetches rely on that cookie
 * being attached automatically. The previous design persisted the Logto
 * access token in localStorage (XSS-stealable) and discarded the refresh
 * token; both are fixed — tokens stay server-side, and the refresh token is
 * used there to renew the session transparently.
 *
 * Consumers must NOT send Authorization headers for same-origin API calls;
 * the cookie authenticates them and the server's Origin check (plus
 * SameSite=Strict) provides CSRF protection.
 */

interface User {
  id: string;
  name: string;
  email: string;
  avatar?: string;
}

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

interface AuthContextType extends AuthState {
  login: () => void;
  signUp: () => void;
  logout: () => void;
  handleCallback: (code: string, state: string) => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

// Build-time fallbacks for local dev; production values come from /api/auth/config
// at runtime so images don't need env-specific rebuilds.
const FALLBACK_AUTH_CONFIG: AuthConfig = {
  endpoint: process.env.NEXT_PUBLIC_LOGTO_ENDPOINT || 'http://localhost:3002',
  appId: process.env.NEXT_PUBLIC_LOGTO_APP_ID || '',
  redirectUri: process.env.NEXT_PUBLIC_REDIRECT_URI || 'http://localhost:3001/callback',
};

interface AuthConfig {
  endpoint: string;
  appId: string;
  redirectUri: string;
}

let cachedAuthConfig: AuthConfig | null = null;

async function getAuthConfig(): Promise<AuthConfig> {
  if (cachedAuthConfig) return cachedAuthConfig;
  try {
    const response = await fetch('/api/auth/config');
    if (response.ok) {
      const config = (await response.json()) as AuthConfig;
      if (config.endpoint && config.appId) {
        cachedAuthConfig = config;
        return config;
      }
    }
  } catch {
    // fall through to build-time values
  }
  return FALLBACK_AUTH_CONFIG;
}

// PKCE helpers
async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const digest = await crypto.subtle.digest('SHA-256', data);
  return base64UrlEncode(digest);
}

function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function generateRandomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~';
  let result = '';
  const randomValues = new Uint8Array(length);
  crypto.getRandomValues(randomValues);
  for (let i = 0; i < length; i++) {
    result += chars[randomValues[i] % chars.length];
  }
  return result;
}

// The PKCE state/verifier are one-time, short-lived values: sessionStorage
// (cleared with the tab) instead of the persistent localStorage the previous
// implementation used.
const OAUTH_STATE_KEY = 'oauth_state';
const OAUTH_VERIFIER_KEY = 'code_verifier';

export function AuthProvider({ children }: { children: ReactNode }) {
  const router = useRouter();
  const [authState, setAuthState] = useState<AuthState>({
    user: null,
    isAuthenticated: false,
    isLoading: true,
  });

  // Check for existing session on mount (HttpOnly cookie; no client tokens)
  useEffect(() => {
    checkSession();
  }, []);

  const checkSession = async () => {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch('/api/auth/me', {
        credentials: 'same-origin',
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (response.ok) {
        const data = await response.json();
        setAuthState({
          user: data.user ?? data,
          isAuthenticated: true,
          isLoading: false,
        });
      } else {
        setAuthState((prev) => ({ ...prev, isLoading: false }));
      }
    } catch (error) {
      console.error('Session check failed:', error);
      setAuthState((prev) => ({ ...prev, isLoading: false }));
    }
  };

  const initiateAuth = useCallback(async (isSignUp: boolean = false) => {
    const config = await getAuthConfig();

    // Clear stale flow state and any legacy tokens before starting a new flow
    try {
      localStorage.removeItem('accessToken');
      sessionStorage.removeItem(OAUTH_STATE_KEY);
      sessionStorage.removeItem(OAUTH_VERIFIER_KEY);
    } catch {
      // storage unavailable (private mode etc.) - flow values below still work
    }

    // Generate state for CSRF protection
    const state = generateRandomString(32);
    sessionStorage.setItem(OAUTH_STATE_KEY, state);

    // Generate PKCE code verifier (43-128 characters per RFC 7636)
    const codeVerifier = generateRandomString(64);
    sessionStorage.setItem(OAUTH_VERIFIER_KEY, codeVerifier);

    // Generate S256 code challenge
    const codeChallenge = await generateCodeChallenge(codeVerifier);

    // Build authorization URL
    const params = new URLSearchParams({
      client_id: config.appId,
      redirect_uri: config.redirectUri,
      response_type: 'code',
      scope: 'openid profile email offline_access',
      state,
      code_challenge: codeChallenge,
      code_challenge_method: 'S256',
    });

    // Add interaction mode for sign-up flow
    if (isSignUp) {
      params.set('first_screen', 'register');
    }

    window.location.href = `${config.endpoint}/oidc/auth?${params.toString()}`;
  }, []);

  const login = useCallback(() => {
    initiateAuth(false);
  }, [initiateAuth]);

  const signUp = useCallback(() => {
    initiateAuth(true);
  }, [initiateAuth]);

  const handleCallback = useCallback(
    async (code: string, state: string) => {
      // Verify state
      const savedState = sessionStorage.getItem(OAUTH_STATE_KEY);
      if (state !== savedState) {
        throw new Error('Invalid state parameter');
      }

      const codeVerifier = sessionStorage.getItem(OAUTH_VERIFIER_KEY);
      if (!codeVerifier) {
        throw new Error('Missing code verifier');
      }

      const config = await getAuthConfig();

      // Exchange code for tokens; the server stores them and sets the
      // HttpOnly session cookie. No token is returned to the browser.
      const response = await fetch('/api/auth/callback', {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          code,
          codeVerifier,
          redirectUri: config.redirectUri,
        }),
      });

      sessionStorage.removeItem(OAUTH_STATE_KEY);
      sessionStorage.removeItem(OAUTH_VERIFIER_KEY);

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || 'Token exchange failed');
      }

      const { user } = await response.json();
      if (!user) {
        throw new Error('Token exchange returned no user');
      }

      setAuthState({
        user,
        isAuthenticated: true,
        isLoading: false,
      });

      router.push('/dashboard');
    },
    [router]
  );

  const logout = useCallback(async () => {
    const config = await getAuthConfig();

    // Revoke the server-side session (Logto refresh token + session row) and
    // clear the session cookie. The Origin header browsers send on POSTs makes
    // this CSRF-safe server-side.
    try {
      await fetch('/api/auth/logout', {
        method: 'POST',
        credentials: 'same-origin',
      });
    } catch {
      // proceed with local cleanup and IdP logout regardless
    }

    setAuthState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
    });

    // Redirect to Logto's end session endpoint to clear the IdP session
    // This ensures the user can log in as a different user next time
    const postLogoutRedirectUri = typeof window !== 'undefined' ? window.location.origin : '';
    const endSessionUrl = new URL(`${config.endpoint}/oidc/session/end`);
    endSessionUrl.searchParams.set('client_id', config.appId);
    endSessionUrl.searchParams.set('post_logout_redirect_uri', postLogoutRedirectUri);

    window.location.href = endSessionUrl.toString();
  }, []);

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
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
