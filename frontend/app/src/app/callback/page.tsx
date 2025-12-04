'use client'

import { useEffect, useState } from 'react'
import { useSearchParams, useRouter } from 'next/navigation'
import { useAuth } from '../hooks/useAuth'
import { BookOpen } from 'lucide-react'

export default function Callback() {
  const searchParams = useSearchParams()
  const router = useRouter()
  const { handleCallback } = useAuth()
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const code = searchParams.get('code')
    const state = searchParams.get('state')
    const errorParam = searchParams.get('error')
    const errorDescription = searchParams.get('error_description')

    if (errorParam) {
      setError(errorDescription || errorParam)
      return
    }

    if (!code || !state) {
      setError('Missing authorization code or state')
      return
    }

    handleCallback(code, state).catch((err) => {
      console.error('Callback error:', err)
      setError(err.message || 'Authentication failed')
    })
  }, [searchParams, handleCallback])

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="card max-w-md w-full text-center">
          <div className="h-16 w-16 rounded-full bg-red-500/20 flex items-center justify-center mx-auto mb-4">
            <span className="text-red-500 text-2xl">!</span>
          </div>
          <h1 className="text-xl font-semibold mb-2">Authentication Failed</h1>
          <p className="text-slate-400 mb-6">{error}</p>
          <button
            onClick={() => router.push('/')}
            className="btn-primary"
          >
            Return Home
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="text-center">
        <div className="relative mb-8">
          <div className="h-16 w-16 rounded-full bg-indigo-500/20 flex items-center justify-center mx-auto animate-pulse">
            <BookOpen className="h-8 w-8 text-indigo-500" />
          </div>
          <div className="absolute inset-0 rounded-full border-2 border-indigo-500/50 animate-ping"></div>
        </div>
        <h1 className="text-xl font-semibold mb-2">Signing you in...</h1>
        <p className="text-slate-400">Please wait while we complete your authentication.</p>
      </div>
    </div>
  )
}

