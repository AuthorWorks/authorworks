import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'

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

// GET /api/books/[id]/chapters/[chapterId]
export async function GET(
  request: NextRequest,
  { params }: { params: { id: string; chapterId: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    // Verify user owns the book
    const bookCheck = await pool.query(
      'SELECT id FROM books WHERE id = $1 AND user_id = $2',
      [params.id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const result = await pool.query(
      'SELECT * FROM chapters WHERE id = $1 AND book_id = $2',
      [params.chapterId, params.id]
    )

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }

    return NextResponse.json(result.rows[0])
  } catch (error) {
    console.error('Error fetching chapter:', error)
    return NextResponse.json({ error: 'Failed to fetch chapter' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// PUT /api/books/[id]/chapters/[chapterId]
export async function PUT(
  request: NextRequest,
  { params }: { params: { id: string; chapterId: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    // Verify user owns the book
    const bookCheck = await pool.query(
      'SELECT id FROM books WHERE id = $1 AND user_id = $2',
      [params.id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const body = await request.json()
    const { title, content } = body

    // Calculate word count
    const wordCount = content ? content.trim().split(/\s+/).filter(Boolean).length : 0

    const result = await pool.query(
      `UPDATE chapters
       SET title = COALESCE($1, title),
           content = COALESCE($2, content),
           word_count = $3,
           updated_at = NOW()
       WHERE id = $4 AND book_id = $5
       RETURNING *`,
      [title, content, wordCount, params.chapterId, params.id]
    )

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }

    // Update book's total word count
    await pool.query(
      `UPDATE books
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM chapters WHERE book_id = $1),
           updated_at = NOW()
       WHERE id = $1`,
      [params.id]
    )

    return NextResponse.json(result.rows[0])
  } catch (error) {
    console.error('Error updating chapter:', error)
    return NextResponse.json({ error: 'Failed to update chapter' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// DELETE /api/books/[id]/chapters/[chapterId]
export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string; chapterId: string } }
) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    // Verify user owns the book
    const bookCheck = await pool.query(
      'SELECT id FROM books WHERE id = $1 AND user_id = $2',
      [params.id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const result = await pool.query(
      'DELETE FROM chapters WHERE id = $1 AND book_id = $2 RETURNING id',
      [params.chapterId, params.id]
    )

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }

    // Renumber remaining chapters
    await pool.query(
      `WITH numbered AS (
        SELECT id, ROW_NUMBER() OVER (ORDER BY chapter_number) as new_num
        FROM chapters WHERE book_id = $1
      )
      UPDATE chapters c SET chapter_number = n.new_num
      FROM numbered n WHERE c.id = n.id`,
      [params.id]
    )

    // Update book's word count
    await pool.query(
      `UPDATE books
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM chapters WHERE book_id = $1),
           updated_at = NOW()
       WHERE id = $1`,
      [params.id]
    )

    return NextResponse.json({ success: true })
  } catch (error) {
    console.error('Error deleting chapter:', error)
    return NextResponse.json({ error: 'Failed to delete chapter' }, { status: 500 })
  } finally {
    await pool.end()
  }
}
