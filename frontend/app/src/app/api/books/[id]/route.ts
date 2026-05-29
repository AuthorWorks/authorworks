import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

interface RouteContext {
  params: { id: string }
}

const UPDATABLE_FIELDS = ['title', 'description', 'genre', 'status', 'cover_image_url', 'metadata'] as const
type UpdatableField = (typeof UPDATABLE_FIELDS)[number]

// GET /api/books/[id] - Fetch a single book owned by the user.
export async function GET(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const result = await pool.query(
      `SELECT * FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [params.id, userId]
    )
    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }
    return NextResponse.json(result.rows[0])
  } catch (error) {
    console.error('GET /api/books/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to fetch book' }, { status: 500 })
  }
}

// PUT /api/books/[id] - Update mutable fields on a book.
export async function PUT(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: Partial<Record<UpdatableField, unknown>>
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const updates: string[] = ['updated_at = NOW()']
  const values: unknown[] = []
  for (const field of UPDATABLE_FIELDS) {
    if (body[field] === undefined) continue
    values.push(field === 'metadata' ? JSON.stringify(body[field]) : body[field])
    updates.push(`${field} = $${values.length}`)
  }

  if (values.length === 0) {
    return NextResponse.json({ error: 'No updatable fields provided' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    values.push(params.id, userId)
    const result = await pool.query(
      `UPDATE ${booksTable} SET ${updates.join(', ')}
         WHERE id = $${values.length - 1} AND ${bookOwnerCol} = $${values.length}
         RETURNING *`,
      values
    )
    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }
    return NextResponse.json(result.rows[0])
  } catch (error) {
    console.error('PUT /api/books/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to update book' }, { status: 500 })
  }
}

// DELETE /api/books/[id] - Delete a book and cascade owned chapters/logs.
export async function DELETE(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const result = await pool.query(
      `DELETE FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2 RETURNING id`,
      [params.id, userId]
    )
    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }
    return NextResponse.json({ success: true })
  } catch (error) {
    console.error('DELETE /api/books/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to delete book' }, { status: 500 })
  }
}
