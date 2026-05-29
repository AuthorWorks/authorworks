import { NextRequest, NextResponse } from 'next/server'
import { resolveUser, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'

const ENSURE_TABLE_SQL = `
  CREATE TABLE IF NOT EXISTS public.user_profiles (
    user_id      VARCHAR(255) PRIMARY KEY,
    name         TEXT,
    bio          TEXT,
    website      TEXT,
    avatar_url   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
  )
`

let tableEnsured = false
async function ensureProfileTable(): Promise<void> {
  if (tableEnsured) return
  await getPool().query(ENSURE_TABLE_SQL)
  tableEnsured = true
}

// GET /api/users/profile - Returns the current user's profile, merging Logto identity with stored fields.
export async function GET(request: NextRequest) {
  const user = await resolveUser(request)
  if (!user) return unauthorized()

  try {
    await ensureProfileTable()
    const result = await getPool().query(
      `SELECT name, bio, website, avatar_url, created_at, updated_at
         FROM public.user_profiles WHERE user_id = $1`,
      [user.sub]
    )
    const stored = result.rows[0] ?? {}
    return NextResponse.json({
      user_id: user.sub,
      email: user.email ?? null,
      name: stored.name ?? user.name ?? user.username ?? null,
      bio: stored.bio ?? null,
      website: stored.website ?? null,
      avatar_url: stored.avatar_url ?? user.picture ?? null,
      created_at: stored.created_at ?? null,
      updated_at: stored.updated_at ?? null,
    })
  } catch (error) {
    console.error('GET /api/users/profile failed:', error)
    return NextResponse.json({ error: 'Failed to fetch profile' }, { status: 500 })
  }
}

// PUT /api/users/profile - Upserts the user's profile fields.
export async function PUT(request: NextRequest) {
  const user = await resolveUser(request)
  if (!user) return unauthorized()

  let body: { name?: string | null; bio?: string | null; website?: string | null; avatar_url?: string | null }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  try {
    await ensureProfileTable()
    const result = await getPool().query(
      `INSERT INTO public.user_profiles (user_id, name, bio, website, avatar_url)
         VALUES ($1, $2, $3, $4, $5)
       ON CONFLICT (user_id) DO UPDATE SET
         name = COALESCE(EXCLUDED.name, public.user_profiles.name),
         bio = COALESCE(EXCLUDED.bio, public.user_profiles.bio),
         website = COALESCE(EXCLUDED.website, public.user_profiles.website),
         avatar_url = COALESCE(EXCLUDED.avatar_url, public.user_profiles.avatar_url),
         updated_at = NOW()
       RETURNING name, bio, website, avatar_url, created_at, updated_at`,
      [user.sub, body.name ?? null, body.bio ?? null, body.website ?? null, body.avatar_url ?? null]
    )
    const stored = result.rows[0]
    return NextResponse.json({
      user_id: user.sub,
      email: user.email ?? null,
      ...stored,
    })
  } catch (error) {
    console.error('PUT /api/users/profile failed:', error)
    return NextResponse.json({ error: 'Failed to update profile' }, { status: 500 })
  }
}
