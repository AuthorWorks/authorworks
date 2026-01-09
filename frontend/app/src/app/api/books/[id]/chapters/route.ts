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

// GET /api/books/[id]/chapters - List chapters for a book
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
    // First verify user owns the book
    const bookCheck = await pool.query(
      'SELECT id FROM books WHERE id = $1 AND user_id = $2',
      [params.id, userId]
    )

    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const result = await pool.query(
      'SELECT * FROM chapters WHERE book_id = $1 ORDER BY chapter_number ASC',
      [params.id]
    )

    return NextResponse.json({ chapters: result.rows })
  } catch (error) {
    console.error('Error fetching chapters:', error)
    return NextResponse.json({ error: 'Failed to fetch chapters' }, { status: 500 })
  } finally {
    await pool.end()
  }
}

// POST /api/books/[id]/chapters - Create a new chapter
export async function POST(
  request: NextRequest,
  { params }: { params: { id: string } }
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

    // Get next chapter number
    const maxResult = await pool.query(
      'SELECT COALESCE(MAX(chapter_number), 0) + 1 as next_num FROM chapters WHERE book_id = $1',
      [params.id]
    )
    const nextChapterNumber = maxResult.rows[0].next_num

    const result = await pool.query(
      `INSERT INTO chapters (book_id, chapter_number, title, content, word_count)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [params.id, nextChapterNumber, title || null, content || '', content ? content.split(/\s+/).length : 0]
    )

    return NextResponse.json(result.rows[0], { status: 201 })
  } catch (error) {
    console.error('Error creating chapter:', error)
    return NextResponse.json({ error: 'Failed to create chapter' }, { status: 500 })
  } finally {
    await pool.end()
  }
}
