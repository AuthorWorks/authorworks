import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

const VALID_STATUSES = new Set(['draft', 'writing', 'editing', 'published', 'archived'])

// GET /api/books - List books owned by the authenticated user.
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const { searchParams } = new URL(request.url)
  const limit = Math.min(Math.max(parseInt(searchParams.get('limit') || '50', 10) || 50, 1), 500)
  const status = searchParams.get('status')
  if (status && !VALID_STATUSES.has(status)) {
    return NextResponse.json({ error: 'Invalid status filter' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)

    const params: unknown[] = [userId]
    let query = `SELECT id, ${bookOwnerCol} AS user_id, title, description, genre, status,
                    word_count, metadata, created_at, updated_at
             FROM ${booksTable} WHERE ${bookOwnerCol} = $1`
    if (status) {
      params.push(status)
      query += ` AND status = $${params.length}`
    }
    params.push(limit)
    query += ` ORDER BY updated_at DESC LIMIT $${params.length}`

    const result = await pool.query(query, params)
    return NextResponse.json({ books: result.rows })
  } catch (error) {
    console.error('GET /api/books failed:', error)
    return NextResponse.json({ error: 'Failed to fetch books' }, { status: 500 })
  }
}

// POST /api/books - Create a new book for the authenticated user.
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: { title?: string; description?: string; genre?: string; metadata?: unknown }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const title = body.title?.trim()
  if (!title) {
    return NextResponse.json({ error: 'Title is required' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const isContentSchema = booksTable.startsWith('content.')
    const metadataJson = body.metadata ? JSON.stringify(body.metadata) : '{}'

    const result = isContentSchema
      ? await pool.query(
          `INSERT INTO ${booksTable}
             (id, ${bookOwnerCol}, title, description, genre, status, metadata, created_at, updated_at)
           VALUES (gen_random_uuid(), $1::uuid, $2, $3, $4, 'draft', $5, NOW(), NOW())
           RETURNING id, ${bookOwnerCol} AS user_id, title, description, genre, status, metadata, created_at, updated_at`,
          [userId, title, body.description ?? null, body.genre ?? null, metadataJson]
        )
      : await pool.query(
          `INSERT INTO ${booksTable}
             (${bookOwnerCol}, title, description, genre, metadata)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *`,
          [userId, title, body.description ?? null, body.genre ?? null, metadataJson]
        )

    return NextResponse.json(result.rows[0], { status: 201 })
  } catch (error) {
    console.error('POST /api/books failed:', error)
    return NextResponse.json({ error: 'Failed to create book' }, { status: 500 })
  }
}
