import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'

// Database connection
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

// POST /api/generate/book/sync - Sync generated book data to database
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  const pool = getPool()
  try {
    const body: SyncRequest = await request.json()
    const { book_id, job_id, synopsis, themes, chapters, pdf_path, epub_path } = body

    console.log(`Syncing book ${book_id} from job ${job_id}`)

    // Verify user owns the book
    const bookCheck = await pool.query(
      'SELECT id FROM books WHERE id = $1 AND user_id = $2',
      [book_id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    // Sync chapters to database
    if (chapters && chapters.length > 0) {
      // Delete existing chapters first
      await pool.query('DELETE FROM chapters WHERE book_id = $1', [book_id])

      // Insert new chapters
      for (const chapter of chapters) {
        const content = chapter.content || [
          `## ${chapter.title}`,
          '',
          chapter.summary || ''
        ].join('\n')

        await pool.query(
          `INSERT INTO chapters (book_id, chapter_number, title, content, word_count)
           VALUES ($1, $2, $3, $4, $5)`,
          [book_id, chapter.number, chapter.title, content, content.split(/\s+/).length]
        )
      }

      console.log(`Synced ${chapters.length} chapters for book ${book_id}`)
    }

    // Update book metadata
    const metadataUpdate: Record<string, any> = {
      generation_completed: true,
      generation_job_id: job_id,
    }
    if (synopsis) metadataUpdate.ai_synopsis = synopsis
    if (themes) metadataUpdate.themes = themes
    if (pdf_path) metadataUpdate.pdf_path = pdf_path
    if (epub_path) metadataUpdate.epub_path = epub_path

    await pool.query(
      `UPDATE books SET
         metadata = metadata || $1,
         status = 'draft',
         updated_at = NOW()
       WHERE id = $2`,
      [JSON.stringify(metadataUpdate), book_id]
    )

    // Update book word count
    await pool.query(
      `UPDATE books
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM chapters WHERE book_id = $1)
       WHERE id = $1`,
      [book_id]
    )

    // Update generation log
    await pool.query(
      `UPDATE generation_logs
       SET status = 'completed', completed_at = NOW()
       WHERE book_id = $1 AND result->>'job_id' = $2`,
      [book_id, job_id]
    )

    return NextResponse.json({
      success: true,
      message: 'Book data synced successfully'
    })
  } catch (error) {
    console.error('Error syncing book:', error)
    return NextResponse.json({ error: 'Failed to sync book' }, { status: 500 })
  } finally {
    await pool.end()
  }
}
