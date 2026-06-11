import { NextRequest, NextResponse } from 'next/server'
import type { Pool } from 'pg'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { countWords } from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

const DEFAULT_BOOK_GENERATOR_URL =
  'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'

interface JobStatus {
  status: string
  book_id?: string
  error?: string
  synced?: boolean
  synopsis?: string
  themes?: string[]
  chapters?: Array<{ title?: string; content?: string; summary?: string }>
  outline?: { chapters?: Array<{ title?: string; content?: string; summary?: string }> }
}

// GET /api/generate/book/status/:jobId - Polls the book generator and auto-syncs on completion.
export async function GET(
  request: NextRequest,
  { params }: { params: { jobId: string } }
) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const generatorUrl = process.env.BOOK_GENERATOR_URL || DEFAULT_BOOK_GENERATOR_URL
  const pool = getPool()

  let response: Response
  try {
    response = await fetch(`${generatorUrl}/api/jobs/${params.jobId}`, {
      headers: { 'Content-Type': 'application/json' },
      cache: 'no-store',
    })
  } catch (error) {
    console.error('Job status fetch failed:', error)
    await markGenerationLogFailed(pool, params.jobId, 'Internal status polling error')
    return NextResponse.json({ error: 'Internal server error' }, { status: 500 })
  }

  if (!response.ok) {
    if (response.status === 404) {
      await markGenerationLogFailed(pool, params.jobId, 'Job not found in book generator')
      return NextResponse.json({ error: 'Job not found' }, { status: 404 })
    }
    await markGenerationLogFailed(pool, params.jobId, 'Failed to get job status from book generator')
    return NextResponse.json({ error: 'Failed to get job status' }, { status: 500 })
  }

  const status = (await response.json()) as JobStatus

  // Job statuses include full chapter content - only the book owner may read them.
  if (status.book_id) {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const owned = await pool.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [status.book_id, userId]
    )
    if (owned.rows.length === 0) {
      return NextResponse.json({ error: 'Job not found' }, { status: 404 })
    }
  }

  if (status.status === 'completed' && status.book_id && !status.synced) {
    await autoSyncBook(pool, status.book_id, params.jobId, status)
    status.synced = true
  } else if (status.status === 'failed') {
    await markGenerationLogFailed(pool, params.jobId, status.error || 'Book generation failed')
    if (status.book_id) {
      await updateBookGenerationStatus(pool, status.book_id, {
        generation_status: 'failed',
        generation_error: status.error || 'Book generation failed',
      })
    }
  }

  return NextResponse.json(status)
}

async function updateBookGenerationStatus(
  pool: Pool,
  bookId: string,
  patch: Record<string, unknown>
) {
  try {
    const { booksTable } = await getContentSchemaTables(pool)
    await pool.query(
      `UPDATE ${booksTable} SET
         metadata = COALESCE(metadata, '{}'::jsonb) || $1,
         updated_at = NOW()
       WHERE id = $2`,
      [JSON.stringify(patch), bookId]
    )
  } catch (error) {
    console.warn(`Failed to update generation status for book ${bookId}:`, error)
  }
}

async function autoSyncBook(pool: Pool, bookId: string, jobId: string, status: JobStatus) {
  try {
    const { booksTable, chaptersTable } = await getContentSchemaTables(pool)
    const chapters = status.chapters || status.outline?.chapters || []

    if (chapters.length > 0) {
      await pool.query(`DELETE FROM ${chaptersTable} WHERE book_id = $1`, [bookId])
      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i]
        const content = chapter.content || chapter.summary || ''
        const title = chapter.title || `Chapter ${i + 1}`
        await pool.query(
          `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count, status)
           VALUES ($1, $2, $3, $4, $5, 'draft')`,
          [bookId, i + 1, title, content, countWords(content)]
        )
      }
    }

    const metadata: Record<string, unknown> = {
      generation_completed: true,
      generation_status: 'completed',
      generation_job_id: jobId,
      completed_at: new Date().toISOString(),
    }
    if (status.synopsis) metadata.ai_synopsis = status.synopsis
    if (status.themes) metadata.themes = status.themes

    await pool.query(
      `UPDATE ${booksTable} SET
         metadata = COALESCE(metadata, '{}'::jsonb) || $1,
         status = 'draft',
         updated_at = NOW(),
         word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $2)
       WHERE id = $2`,
      [JSON.stringify(metadata), bookId]
    )

    await pool.query(
      `UPDATE generation_logs
         SET status = 'completed', completed_at = NOW()
       WHERE book_id = $1 AND result->>'job_id' = $2`,
      [bookId, jobId]
    )
  } catch (error) {
    console.error(`autoSyncBook failed for book ${bookId} / job ${jobId}:`, error)
  }
}

async function markGenerationLogFailed(pool: Pool, jobId: string, errorMessage: string) {
  try {
    await pool.query(
      `UPDATE generation_logs
         SET status = 'failed', error = $1, completed_at = NOW()
       WHERE result->>'job_id' = $2 AND status = 'pending'`,
      [errorMessage, jobId]
    )
  } catch (error) {
    console.warn(`Failed to mark generation_logs failed for job ${jobId}:`, error)
  }
}
