import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { countWords } from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

interface SyncRequest {
  book_id: string
  job_id: string
  synopsis?: string
  themes?: string[]
  chapters?: Array<{
    number: number
    title: string
    summary?: string
    content?: string
  }>
  pdf_path?: string
  epub_path?: string
}

// POST /api/generate/book/sync - Persist generated chapters/metadata into the database.
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: SyncRequest
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  if (!body.book_id || !body.job_id) {
    return NextResponse.json({ error: 'book_id and job_id are required' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const bookCheck = await pool.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [body.book_id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    if (body.chapters?.length) {
      await pool.query(`DELETE FROM ${chaptersTable} WHERE book_id = $1`, [body.book_id])
      for (const chapter of body.chapters) {
        const content =
          chapter.content || [`## ${chapter.title}`, '', chapter.summary || ''].join('\n')
        await pool.query(
          `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count)
           VALUES ($1, $2, $3, $4, $5)`,
          [body.book_id, chapter.number, chapter.title, content, countWords(content)]
        )
      }
    }

    const metadataUpdate: Record<string, unknown> = {
      generation_completed: true,
      generation_job_id: body.job_id,
    }
    if (body.synopsis) metadataUpdate.ai_synopsis = body.synopsis
    if (body.themes) metadataUpdate.themes = body.themes
    if (body.pdf_path) metadataUpdate.pdf_path = body.pdf_path
    if (body.epub_path) metadataUpdate.epub_path = body.epub_path

    await pool.query(
      `UPDATE ${booksTable} SET
         metadata = COALESCE(metadata, '{}'::jsonb) || $1,
         status = 'draft',
         updated_at = NOW(),
         word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $2)
       WHERE id = $2`,
      [JSON.stringify(metadataUpdate), body.book_id]
    )

    await pool.query(
      `UPDATE generation_logs
         SET status = 'completed', completed_at = NOW()
       WHERE book_id = $1 AND result->>'job_id' = $2`,
      [body.book_id, body.job_id]
    )

    return NextResponse.json({ success: true, message: 'Book data synced successfully' })
  } catch (error) {
    console.error('POST /api/generate/book/sync failed:', error)
    return NextResponse.json({ error: 'Failed to sync book' }, { status: 500 })
  }
}
