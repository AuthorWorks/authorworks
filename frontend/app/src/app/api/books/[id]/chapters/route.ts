import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { countWords } from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

interface RouteContext {
  params: { id: string }
}

// GET /api/books/[id]/chapters - List chapters in order for the given book.
export async function GET(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const owned = await pool.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [params.id, userId]
    )
    if (owned.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const result = await pool.query(
      `SELECT * FROM ${chaptersTable} WHERE book_id = $1 ORDER BY chapter_number ASC`,
      [params.id]
    )
    return NextResponse.json({ chapters: result.rows })
  } catch (error) {
    console.error('GET /api/books/[id]/chapters failed:', error)
    return NextResponse.json({ error: 'Failed to fetch chapters' }, { status: 500 })
  }
}

// POST /api/books/[id]/chapters - Create a new chapter at the end of the book.
export async function POST(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: { title?: string | null; content?: string | null }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const owned = await pool.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [params.id, userId]
    )
    if (owned.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    const next = await pool.query(
      `SELECT COALESCE(MAX(chapter_number), 0) + 1 AS next_num FROM ${chaptersTable} WHERE book_id = $1`,
      [params.id]
    )
    const nextNumber = next.rows[0].next_num

    const content = body.content ?? ''
    const result = await pool.query(
      `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count)
       VALUES ($1, $2, $3, $4, $5) RETURNING *`,
      [params.id, nextNumber, body.title ?? null, content, countWords(content)]
    )
    return NextResponse.json(result.rows[0], { status: 201 })
  } catch (error) {
    console.error('POST /api/books/[id]/chapters failed:', error)
    return NextResponse.json({ error: 'Failed to create chapter' }, { status: 500 })
  }
}
