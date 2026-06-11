import { NextRequest, NextResponse } from 'next/server'
import { resolveUser, toClientUser } from '@/app/lib/auth'

// GET /api/auth/me - Returns the authenticated Logto user, or 401 when missing/invalid.
export async function GET(request: NextRequest) {
  const user = await resolveUser(request)
  if (!user) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }
  return NextResponse.json({ user: toClientUser(user) })
}
