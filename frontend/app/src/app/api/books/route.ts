import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'

// Database connection pool - read inside handler to avoid build-time caching
function getPool() {
  return new Pool({
    connectionString: process.env.DATABASE_URL || 'postgresql://postgres:homelab_postgres_2024@postgres.databases.svc.cluster.local:5432/authorworks',
  })
}

// Helper to get user ID from auth token
async function getUserId(request: NextRequest): Promise<string | null> {
  const authHeader = request.headers.get('authorization')
  if (!authHeader?.startsWith('Bearer ')) return null

  const token = authHeader.substring(7)
  const LOGTO_ENDPOINT = process.env.LOGTO_ENDPOINT || 'http://logto.security.svc.cluster.local:3001'

  try {
    const response = await fetch(`${LOGTO_ENDPOINT}/oidc/me`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!response.ok) return null
    const userInfo = await response.json()
    return userInfo.sub
  } catch {
    return null
  }
}

// GET /api/books - List all books for the user
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    const { searchParams } = new URL(request.url)
    const limit = parseInt(searchParams.get('limit') || '50')
    const status = searchParams.get('status')

    let query = 'SELECT * FROM books WHERE user_id = $1'
    const params: any[] = [userId]

    if (status) {
      query += ' AND status = $2'
      params.push(status)
    }

    query += ' ORDER BY updated_at DESC LIMIT $' + (params.length + 1)
    params.push(limit)

    const result = await pool.query(query, params)

    return NextResponse.json({ books: result.rows })
  } catch (error) {
    console.error('Error fetching books:', error)
    return NextResponse.json({ error: 'Failed to fetch books' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// POST /api/books - Create a new book
export async function POST(request: NextRequest) {
  console.log('POST /api/books - starting')

  const userId = await getUserId(request)
  console.log('POST /api/books - userId:', userId)

  if (!userId) {
    console.log('POST /api/books - Unauthorized, no userId')
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    const body = await request.json()
    console.log('POST /api/books - body:', JSON.stringify(body))
    const { title, description, genre, metadata } = body

    if (!title?.trim()) {
      console.log('POST /api/books - Missing title')
      return NextResponse.json({ error: 'Title is required' }, { status: 400 })
    }

    console.log('POST /api/books - Inserting into database')
    const result = await pool.query(
      `INSERT INTO books (user_id, title, description, genre, metadata)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [userId, title.trim(), description || null, genre || null, metadata ? JSON.stringify(metadata) : '{}']
    )

    console.log('POST /api/books - Success, book id:', result.rows[0]?.id)
    return NextResponse.json(result.rows[0], { status: 201 })
  } catch (error) {
    console.error('POST /api/books - Error:', error)
    return NextResponse.json({ error: 'Failed to create book' }, { status: 500 })
  } finally {
    await pool.end()
  }
}
