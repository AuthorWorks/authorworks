import type { Pool, PoolClient } from 'pg'
import { countWords } from './chapters'
import { getContentSchemaTables, type ContentSchemaTables } from './db-schema'

/**
 * Manuscript-safe generation completion.
 *
 * Invariants enforced here:
 *  - Author text is NEVER automatically replaced without a trustworthy
 *    generation-start baseline (chapter fingerprint recorded when the job was
 *    kicked off) plus explicit conflict handling.
 *  - One transaction / one pooled client per completion; all writes commit or
 *    roll back together, so a partial apply never reports success.
 *  - A durable per-job application record (`generation_completions`) makes each
 *    job apply at most once; the record is bound to book + owner and jobs that
 *    belong to someone/something else fail closed.
 *  - Author saves and completions serialize on the same per-book advisory
 *    lock (`pg_advisory_xact_lock`), so a completion cannot interleave with a
 *    chapter edit and re-read a half-written manuscript.
 *  - Legacy / ambiguous / stale jobs never touch current chapters; their
 *    output is retained separately as a pending proposal for the author.
 */

import { acquireBookLock } from './book-lock'

export { acquireBookLock }

export interface GeneratedChapter {
  title?: string | null
  content?: string | null
  summary?: string | null
}

export interface CompletionOutput {
  chapters: GeneratedChapter[]
  synopsis?: string | null
  themes?: string[] | null
  pdfPath?: string | null
  epubPath?: string | null
}

export interface ChapterSnapshot {
  id: string
  chapter_number: number
  updated_at: Date | string
}

export type CompletionOutcome =
  | 'applied'
  | 'already_applied'
  | 'pending_author_approval'
  | 'rejected_stale'
  | 'rejected_bound'
  | 'not_found'
  | 'error'

export interface CompletionResult {
  outcome: CompletionOutcome
  reason?: string
  jobId: string
  bookId: string
  /** True only when the manuscript (or a proposal) is durably recorded. */
  persisted: boolean
}

export interface PendingProposal {
  job_id: string
  reason: 'author_edits_since_start' | 'legacy_no_baseline' | string
  synopsis?: string | null
  themes?: string[] | null
  chapters: Array<{ number: number; title: string; content: string }>
}

/** Returns the owned book row (id, metadata, status), or null. Fails closed. */
export async function getOwnedBook(
  client: PoolClient,
  tables: ContentSchemaTables,
  bookId: string,
  userId: string
): Promise<{ id: string; metadata: Record<string, unknown> } | null> {
  const result = await client.query(
    `SELECT id, COALESCE(metadata, '{}'::jsonb) AS metadata
       FROM ${tables.booksTable}
      WHERE id = $1 AND ${tables.bookOwnerCol} = $2`,
    [bookId, userId]
  )
  const row = result.rows[0]
  if (!row) return null
  return { id: row.id, metadata: row.metadata ?? {} }
}

/** Ordered chapter identity snapshot used for baseline fingerprints. */
export async function snapshotChapters(
  client: PoolClient,
  tables: ContentSchemaTables,
  bookId: string
): Promise<ChapterSnapshot[]> {
  const result = await client.query(
    `SELECT id, chapter_number, updated_at
       FROM ${tables.chaptersTable}
      WHERE book_id = $1
      ORDER BY chapter_number ASC`,
    [bookId]
  )
  return result.rows.map((r: { id: string; chapter_number: number; updated_at: Date }) => ({
    id: r.id,
    chapter_number: Number(r.chapter_number),
    updated_at: r.updated_at,
  }))
}

export function fingerprintChapters(snapshot: ChapterSnapshot[]): string {
  return snapshot
    .map(
      (c) =>
        `${c.id}|${c.chapter_number}|${new Date(c.updated_at).toISOString()}`
    )
    .join('\n')
}

/**
 * Records the generation-start baseline on the book, under the shared book
 * lock, atomically with the caller's job metadata (active job id, running
 * status, start time). Called right after the generator accepts a new job.
 * The completion path compares the live manuscript against this fingerprint;
 * only an exact match proves the author has not touched the chapters since
 * asking for a regeneration, which is what makes replacing them safe.
 */
export async function recordGenerationStart(
  pool: Pool,
  bookId: string,
  jobId: string,
  extraMetadata: Record<string, unknown> = {}
): Promise<void> {
  const tables = await getContentSchemaTables(pool)
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    await acquireBookLock(client, bookId)
    const fingerprint = fingerprintChapters(
      await snapshotChapters(client, tables, bookId)
    )
    const metadata = {
      ...extraMetadata,
      generation_baseline: {
        job_id: jobId,
        recorded_at: new Date().toISOString(),
        fingerprint,
      },
    }
    await client.query(
      `UPDATE ${tables.booksTable}
          SET metadata = COALESCE(metadata, '{}'::jsonb) || $1::jsonb,
              updated_at = NOW()
        WHERE id = $2`,
      [JSON.stringify(metadata), bookId]
    )
    await client.query('COMMIT')
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}

function normalizeGeneratedChapters(output: CompletionOutput): Array<{
  number: number
  title: string
  content: string
}> {
  return output.chapters.map((c, i) => {
    const title = (c.title ?? '').trim() || `Chapter ${i + 1}`
    const content = c.content ?? c.summary ?? ''
    return { number: i + 1, title, content }
  })
}

async function markGenerationLogCompleted(
  client: PoolClient,
  bookId: string,
  jobId: string
): Promise<void> {
  // Best-effort: a missing/legacy log table must not fail the completion.
  try {
    await client.query(
      `UPDATE generation_logs
          SET status = 'completed',
              completed_at = COALESCE(completed_at, NOW()),
              updated_at = NOW()
        WHERE book_id = $1 AND result->>'job_id' = $2`,
      [bookId, jobId]
    )
  } catch (error) {
    console.warn(`generation_logs update skipped for job ${jobId}:`, error)
  }
}

/**
 * Applies a completed generation job to a book, transactionally.
 *
 * Callers must pass AUTHENTICATED owner output fetched from the authoritative
 * generator (never client-submitted chapter payloads) — ownership of the book
 * is re-verified here and the durable completion record binds job→book→owner.
 */
export async function applyGenerationCompletion(
  pool: Pool,
  args: {
    bookId: string
    jobId: string
    userId: string
    output: CompletionOutput
  }
): Promise<CompletionResult> {
  const { bookId, jobId, userId, output } = args
  const tables = await getContentSchemaTables(pool)
  const generated = normalizeGeneratedChapters(output)

  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    await acquireBookLock(client, bookId)

    const book = await getOwnedBook(client, tables, bookId, userId)
    if (!book) {
      await client.query('ROLLBACK')
      return { outcome: 'not_found', jobId, bookId, persisted: false }
    }

    const meta = book.metadata as Record<string, unknown>
    const baseline = meta.generation_baseline as
      | { job_id?: string; fingerprint?: string }
      | undefined
    const activeJob = meta.generation_job_id as string | undefined
    const alreadyCompleted = meta.generation_completed === true

    // Durable per-job application record, bound to this book and owner.
    const claim = await client.query(
      `INSERT INTO generation_completions (job_id, book_id, user_id, status, result)
       VALUES ($1, $2, $3, 'applying', $4::jsonb)
       ON CONFLICT (job_id) DO NOTHING
       RETURNING job_id`,
      [jobId, bookId, userId, JSON.stringify({ chapters: generated, synopsis: output.synopsis ?? null, themes: output.themes ?? null })]
    )
    if (claim.rowCount === 0) {
      const existing = await client.query(
        `SELECT book_id, user_id, status FROM generation_completions WHERE job_id = $1`,
        [jobId]
      )
      const row = existing.rows[0] as
        | { book_id: string; user_id: string; status: string }
        | undefined
      await client.query('ROLLBACK')
      if (!row) {
        return { outcome: 'error', reason: 'completion record vanished', jobId, bookId, persisted: false }
      }
      if (row.book_id !== bookId || row.user_id !== userId) {
        return {
          outcome: 'rejected_bound',
          reason: 'job is bound to a different book or owner',
          jobId,
          bookId,
          persisted: false,
        }
      }
      return { outcome: 'already_applied', jobId, bookId, persisted: row.status === 'applied' || row.status === 'pending_author_approval' }
    }

    const currentFingerprint = fingerprintChapters(
      await snapshotChapters(client, tables, bookId)
    )

    let outcome: CompletionOutcome
    let reason: string | undefined
    const baselineMatchesThisJob =
      baseline?.job_id === jobId && typeof baseline?.fingerprint === 'string'

    if (baselineMatchesThisJob && baseline.fingerprint === currentFingerprint) {
      // Author has not touched the manuscript since requesting generation:
      // safe to replace chapters in place, preserving stable chapter ids.
      outcome = 'applied'
    } else if (baselineMatchesThisJob) {
      outcome = 'pending_author_approval'
      reason = 'author_edits_since_start'
    } else if (activeJob === jobId && !alreadyCompleted) {
      // Job started before baselines were recorded (or start recording failed):
      // ambiguous — never overwrite, keep the output as a proposal.
      outcome = 'pending_author_approval'
      reason = 'legacy_no_baseline'
    } else {
      // Stale job (a newer job exists) or a historical replay of an already
      // completed book: reject without touching the manuscript.
      outcome = 'rejected_stale'
      reason =
        activeJob && activeJob !== jobId
          ? `stale job; book's active generation job is ${activeJob}`
          : alreadyCompleted
            ? 'book already has a completed generation; refusing replay'
            : 'unknown or unbound job'
    }

    if (outcome === 'applied') {
      const existing = await snapshotChapters(client, tables, bookId)
      // The verified baseline proves the current rows are exactly the
      // pre-generation set, so positional mapping is safe here (and only
      // here); matching across jobs by ordinal alone is what this guard
      // prevents. Existing rows keep their ids; surplus rows are removed.
      for (let i = 0; i < generated.length; i++) {
        const gen = generated[i]
        const row = existing[i]
        const wordCount = countWords(gen.content)
        if (row) {
          await client.query(
            `UPDATE ${tables.chaptersTable}
                SET title = $1, content = $2, word_count = $3,
                    status = 'draft', updated_at = NOW()
              WHERE id = $4`,
            [gen.title, gen.content, wordCount, row.id]
          )
        } else {
          await client.query(
            `INSERT INTO ${tables.chaptersTable}
                (book_id, chapter_number, title, content, word_count, status)
             VALUES ($1, $2, $3, $4, $5, 'draft')`,
            [bookId, gen.number, gen.title, gen.content, wordCount]
          )
        }
      }
      for (let i = generated.length; i < existing.length; i++) {
        await client.query(
          `DELETE FROM ${tables.chaptersTable} WHERE id = $1`,
          [existing[i].id]
        )
      }

      await client.query(
        `UPDATE ${tables.booksTable}
            SET metadata = (COALESCE(metadata, '{}'::jsonb) || $1::jsonb) - 'generation_pending',
                status = 'draft',
                updated_at = NOW(),
                word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${tables.chaptersTable} WHERE book_id = $2)
          WHERE id = $2`,
        [
          JSON.stringify({
            generation_completed: true,
            generation_status: 'completed',
            generation_job_id: jobId,
            completed_at: new Date().toISOString(),
            ...(output.synopsis ? { ai_synopsis: output.synopsis } : {}),
            ...(output.themes ? { themes: output.themes } : {}),
            ...(output.pdfPath ? { pdf_path: output.pdfPath } : {}),
            ...(output.epubPath ? { epub_path: output.epubPath } : {}),
          }),
          bookId,
        ]
      )
      await client.query(
        `UPDATE generation_completions
            SET status = 'applied', completed_at = NOW(), updated_at = NOW()
          WHERE job_id = $1`,
        [jobId]
      )
      await markGenerationLogCompleted(client, bookId, jobId)
    } else if (outcome === 'pending_author_approval') {
      // Preserve the author's current chapters; retain the incoming output
      // separately for explicit review.
      const proposal: PendingProposal = {
        job_id: jobId,
        reason: reason ?? 'legacy_no_baseline',
        synopsis: output.synopsis ?? null,
        themes: output.themes ?? null,
        chapters: generated,
      }
      await client.query(
        `UPDATE ${tables.booksTable}
            SET metadata = COALESCE(metadata, '{}'::jsonb) || $1::jsonb,
                updated_at = NOW()
          WHERE id = $2`,
        [
          JSON.stringify({
            generation_pending: proposal,
            generation_status: 'pending_approval',
          }),
          bookId,
        ]
      )
      await client.query(
        `UPDATE generation_completions
            SET status = 'pending_author_approval',
                result = $2::jsonb,
                completed_at = NOW(), updated_at = NOW()
          WHERE job_id = $1`,
        [jobId, JSON.stringify(proposal)]
      )
      await markGenerationLogCompleted(client, bookId, jobId)
    } else {
      // rejected_stale: record the rejection durably, touch nothing else.
      await client.query(
        `UPDATE generation_completions
            SET status = 'rejected_stale', result = $2::jsonb,
                completed_at = NOW(), updated_at = NOW()
          WHERE job_id = $1`,
        [jobId, JSON.stringify({ reason: reason ?? 'stale' })]
      )
    }

    await client.query('COMMIT')
    // All outcomes reachable here (applied / pending_author_approval /
    // rejected_stale) are durably recorded before this COMMIT.
    return { outcome, reason, jobId, bookId, persisted: true }
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}

/**
 * Explicitly accepts (applies over current text) or declines a stored pending
 * proposal. This is the author's conflict-resolution path; it never runs
 * automatically. Returns false when there is no pending proposal for the job.
 */
export async function resolvePendingProposal(
  pool: Pool,
  args: { bookId: string; jobId: string; userId: string; accept: boolean }
): Promise<{ ok: boolean; outcome: 'accepted' | 'declined' | 'not_found' | 'not_pending' }> {
  const { bookId, jobId, userId, accept } = args
  const tables = await getContentSchemaTables(pool)
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    await acquireBookLock(client, bookId)

    const book = await getOwnedBook(client, tables, bookId, userId)
    if (!book) {
      await client.query('ROLLBACK')
      return { ok: false, outcome: 'not_found' }
    }

    const meta = book.metadata as Record<string, unknown>
    const pending = meta.generation_pending as PendingProposal | undefined
    if (!pending || pending.job_id !== jobId) {
      await client.query('ROLLBACK')
      return { ok: false, outcome: 'not_pending' }
    }

    const record = await client.query(
      `SELECT status FROM generation_completions WHERE job_id = $1`,
      [jobId]
    )
    const alreadyResolved =
      (record.rows[0] as { status?: string } | undefined)?.status === 'applied'
    if (alreadyResolved) {
      await client.query('ROLLBACK')
      return { ok: false, outcome: 'not_pending' }
    }

    if (accept) {
      const existing = await snapshotChapters(client, tables, bookId)
      const generated = pending.chapters
      for (let i = 0; i < generated.length; i++) {
        const gen = generated[i]
        const row = existing[i]
        const wordCount = countWords(gen.content)
        if (row) {
          await client.query(
            `UPDATE ${tables.chaptersTable}
                SET title = $1, content = $2, word_count = $3,
                    status = 'draft', updated_at = NOW()
              WHERE id = $4`,
            [gen.title, gen.content, wordCount, row.id]
          )
        } else {
          await client.query(
            `INSERT INTO ${tables.chaptersTable}
                (book_id, chapter_number, title, content, word_count, status)
             VALUES ($1, $2, $3, $4, $5, 'draft')`,
            [bookId, gen.number, gen.title, gen.content, wordCount]
          )
        }
      }
      for (let i = generated.length; i < existing.length; i++) {
        await client.query(`DELETE FROM ${tables.chaptersTable} WHERE id = $1`, [
          existing[i].id,
        ])
      }
      await client.query(
        `UPDATE ${tables.booksTable}
            SET metadata = (COALESCE(metadata, '{}'::jsonb) || $1::jsonb) - 'generation_pending',
                status = 'draft',
                updated_at = NOW(),
                word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${tables.chaptersTable} WHERE book_id = $2)
          WHERE id = $2`,
        [
          JSON.stringify({
            generation_completed: true,
            generation_status: 'completed',
            generation_job_id: jobId,
            completed_at: new Date().toISOString(),
          }),
          bookId,
        ]
      )
      await client.query(
        `UPDATE generation_completions
            SET status = 'applied', completed_at = COALESCE(completed_at, NOW()), updated_at = NOW()
          WHERE job_id = $1`,
        [jobId]
      )
    } else {
      await client.query(
        `UPDATE ${tables.booksTable}
            SET metadata = (COALESCE(metadata, '{}'::jsonb) || $1::jsonb) - 'generation_pending',
                updated_at = NOW()
          WHERE id = $2`,
        [JSON.stringify({ generation_status: 'completed' }), bookId]
      )
      await client.query(
        `UPDATE generation_completions
            SET status = 'declined', completed_at = NOW(), updated_at = NOW()
          WHERE job_id = $1`,
        [jobId]
      )
    }

    await client.query('COMMIT')
    return { ok: true, outcome: accept ? 'accepted' : 'declined' }
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}
