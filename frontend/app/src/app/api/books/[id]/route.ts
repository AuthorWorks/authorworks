import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'
import { getContentSchemaTables } from '@/app/lib/db-schema'

function getPool() {
  return new Pool({
    connectionString: process.env.DATABASE_URL || 'postgresql://postgres:homelab_postgres_2024@postgres.databases.svc.cluster.local:5432/authorworks',
  })
}

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

// GET /api/books/[id] - Get a single book
export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

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
    console.error('Error fetching book:', error)
    return NextResponse.json({ error: 'Failed to fetch book' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// PUT /api/books/[id] - Update a book
export async function PUT(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    const body = await request.json()
    const { title, description, genre, status, cover_image_url, metadata } = body

    // Build update query dynamically
    const updates: string[] = ['updated_at = NOW()']
    const values: any[] = []
    let paramIndex = 1

    if (title !== undefined) {
      updates.push(`title = $${paramIndex++}`)
      values.push(title)
    }
    if (description !== undefined) {
      updates.push(`description = $${paramIndex++}`)
      values.push(description)
    }
    if (genre !== undefined) {
      updates.push(`genre = $${paramIndex++}`)
      values.push(genre)
    }
    if (status !== undefined) {
      updates.push(`status = $${paramIndex++}`)
      values.push(status)
    }
    if (cover_image_url !== undefined) {
      updates.push(`cover_image_url = $${paramIndex++}`)
      values.push(cover_image_url)
    }
    if (metadata !== undefined) {
      updates.push(`metadata = $${paramIndex++}`)
      values.push(JSON.stringify(metadata))
    }

    values.push(params.id, userId)

    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const result = await pool.query(
      `UPDATE ${booksTable} SET ${updates.join(', ')}
       WHERE id = $${paramIndex++} AND ${bookOwnerCol} = $${paramIndex}
       RETURNING *`,
      values
    )

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    return NextResponse.json(result.rows[0])
  } catch (error) {
    console.error('Error updating book:', error)
    return NextResponse.json({ error: 'Failed to update book' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// DELETE /api/books/[id] - Delete a book
export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

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
    console.error('Error deleting book:', error)
    return NextResponse.json({ error: 'Failed to delete book' }, { status: 500 })
  } finally {
    await pool.end()
  }
}
