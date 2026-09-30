import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'
import {
  applyGenerationCompletion,
  type CompletionOutput,
} from '@/app/lib/manuscript'

const DEFAULT_BOOK_GENERATOR_URL =
  'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'

interface SyncRequest {
  book_id: string
  job_id: string
  // Chapters submitted by the client are deliberately IGNORED: the explicit
  // sync must persist the generator's authoritative output, not arbitrary text.
  chapters?: unknown[]
  pdf_path?: string
  epub_path?: string
}

interface GeneratorJob {
  status?: string
  book_id?: string
  error?: string
  synopsis?: string
  themes?: string[]
  chapters?: Array<{ title?: string; content?: string; summary?: string }>
  outline?: { chapters?: Array<{ title?: string; content?: string; summary?: string }> }
}

/**
 * POST /api/generate/book/sync - Explicitly persist a completed generation
 * job for a book owned by the caller.
 *
 * The chapter/metadata output is fetched from the authoritative book generator
 * by job id (and the job must belong to this book); it is then applied through
 * the shared manuscript-safe completion path — the same implementation the
 * status polling route uses: one transaction, durable per-job application,
 * generation-start baseline conflict detection, stable chapter ids, and
 * pending-proposal handling for edited/legacy books.
 */
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
    return NextResponse.json(
      { error: 'book_id and job_id are required' },
      { status: 400 }
    )
  }

  const pool = getPool()
  const generatorUrl = process.env.BOOK_GENERATOR_URL || DEFAULT_BOOK_GENERATOR_URL

  try {
    // Ownership of the book is proven before fetching or mutating any job state.
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const bookCheck = await pool.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [body.book_id, userId]
    )
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }

    // Authoritative output: read the job from the generator, never trust the
    // client payload for chapter content.
    let jobResponse: Response
    try {
      jobResponse = await fetch(`${generatorUrl}/api/jobs/${body.job_id}`, {
        headers: { 'Content-Type': 'application/json' },
        cache: 'no-store',
      })
    } catch (error) {
      console.error('Book generator unreachable during sync:', error)
      return NextResponse.json(
        { error: 'Book generator service is unreachable' },
        { status: 503 }
      )
    }
    if (jobResponse.status === 404) {
      return NextResponse.json({ error: 'Job not found' }, { status: 404 })
    }
    if (!jobResponse.ok) {
      return NextResponse.json(
        { error: 'Failed to fetch job from book generator' },
        { status: 502 }
      )
    }

    const job = (await jobResponse.json()) as GeneratorJob
    // The job must belong to this book; a mismatch fails closed.
    if (!job.book_id || job.book_id !== body.book_id) {
      return NextResponse.json(
        { error: 'Job does not belong to this book' },
        { status: 409 }
      )
    }
    if (job.status !== 'completed') {
      return NextResponse.json(
        { error: `Job is not completed (status: ${job.status ?? 'unknown'})` },
        { status: 409 }
      )
    }

    const output: CompletionOutput = {
      chapters: job.chapters || job.outline?.chapters || [],
      synopsis: job.synopsis ?? null,
      themes: job.themes ?? null,
      pdfPath: body.pdf_path ?? null,
      epubPath: body.epub_path ?? null,
    }

    const applied = await applyGenerationCompletion(pool, {
      bookId: body.book_id,
      jobId: body.job_id,
      userId,
      output,
    })

    switch (applied.outcome) {
      case 'applied':
        return NextResponse.json({
          success: true,
          outcome: 'applied',
          message: 'Generated book synced successfully',
        })
      case 'already_applied':
        return NextResponse.json({
          success: true,
          outcome: 'already_applied',
          message: 'This job was already applied; no changes made',
        })
      case 'pending_author_approval':
        return NextResponse.json(
          {
            success: false,
            outcome: 'pending_author_approval',
            reason: applied.reason,
            message:
              'Current chapters were edited after generation started (or this job predates safety baselines). ' +
              'The generated output is saved as a pending revision; accept or decline it at /api/books/:id/generation-pending.',
          },
          { status: 409 }
        )
      case 'rejected_stale':
      case 'rejected_bound':
        return NextResponse.json(
          { success: false, outcome: applied.outcome, reason: applied.reason },
          { status: 409 }
        )
      default:
        return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }
  } catch (error) {
    console.error('POST /api/generate/book/sync failed:', error)
    // Persistence failed: never report the sync as successful.
    return NextResponse.json({ error: 'Failed to sync book' }, { status: 500 })
  }
}
