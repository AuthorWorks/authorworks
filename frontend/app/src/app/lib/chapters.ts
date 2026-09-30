import type { Pool } from 'pg'
import { acquireBookLock } from './book-lock'
import { getContentSchemaTables } from './db-schema'

/** Counts whitespace-delimited words; returns 0 for empty/whitespace-only content. */
export function countWords(content: string | null | undefined): number {
  if (!content) return 0
  const trimmed = content.trim()
  if (!trimmed) return 0
  return trimmed.split(/\s+/).filter(Boolean).length
}

/** Returns the chapter row when it exists and is owned (via book) by `userId`, else null. */
export async function findChapterForUser(
  pool: Pool,
  chapterId: string,
  userId: string,
  options: { withBookTitle?: boolean } = {}
): Promise<Record<string, unknown> | null> {
  const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
  const selection = options.withBookTitle ? 'c.*, b.title AS book_title' : 'c.*'
  const result = await pool.query(
    `SELECT ${selection}
       FROM ${chaptersTable} c
       JOIN ${booksTable} b ON c.book_id = b.id
      WHERE c.id = $1 AND b.${bookOwnerCol} = $2`,
    [chapterId, userId]
  )
  return result.rows[0] ?? null
}

/**
 * Updates a chapter's title/content and returns the new row, or null when not owned.
 *
 * Runs in a single transaction holding the per-book advisory lock (the same
 * lock generation completion takes), so a save can never interleave with a
 * completion applying generated chapters. Optional `expected_updated_at`
 * provides optimistic concurrency: when provided and stale, the save is
 * rejected with a conflict instead of silently overwriting newer text.
 */
export async function updateChapterForUser(
  pool: Pool,
  chapterId: string,
  userId: string,
  patch: { title?: string | null; content?: string | null },
  options: { expected_updated_at?: string | Date } = {}
): Promise<Record<string, unknown> | null | 'conflict'> {
  const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
  const client = await pool.connect()
  try {
    await client.query('BEGIN')

    const owned = await client.query(
      `SELECT c.* FROM ${chaptersTable} c
         JOIN ${booksTable} b ON c.book_id = b.id
        WHERE c.id = $1 AND b.${bookOwnerCol} = $2`,
      [chapterId, userId]
    )
    const row = owned.rows[0]
    if (!row) {
      await client.query('ROLLBACK')
      return null
    }

    const bookId = row.book_id as string
    await acquireBookLock(client, bookId)

    if (options.expected_updated_at !== undefined) {
      const expected = new Date(options.expected_updated_at).getTime()
      const actual = new Date(row.updated_at).getTime()
      if (Number.isNaN(expected) || expected !== actual) {
        await client.query('ROLLBACK')
        return 'conflict'
      }
    }

    const wordCount = countWords(patch.content as string | null | undefined)
    const result = await client.query(
      `UPDATE ${chaptersTable}
          SET title = COALESCE($1, title),
              content = COALESCE($2, content),
              word_count = $3,
              updated_at = NOW()
        WHERE id = $4
        RETURNING *`,
      [patch.title ?? null, patch.content ?? null, wordCount, chapterId]
    )

    await client.query(
      `UPDATE ${booksTable}
          SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
              updated_at = NOW()
        WHERE id = $1`,
      [bookId]
    )

    await client.query('COMMIT')
    return result.rows[0] ?? null
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}

/**
 * Creates a chapter at the end of a book in one transaction under the shared
 * per-book advisory lock. Returns the new row, or null when not owned.
 */
export async function createChapterForUser(
  pool: Pool,
  bookId: string,
  userId: string,
  body: { title?: string | null; content?: string | null }
): Promise<Record<string, unknown> | null> {
  const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    await acquireBookLock(client, bookId)

    const owned = await client.query(
      `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [bookId, userId]
    )
    if (owned.rows.length === 0) {
      await client.query('ROLLBACK')
      return null
    }

    const next = await client.query(
      `SELECT COALESCE(MAX(chapter_number), 0) + 1 AS next_num FROM ${chaptersTable} WHERE book_id = $1`,
      [bookId]
    )
    const content = body.content ?? ''
    const result = await client.query(
      `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count)
       VALUES ($1, $2, $3, $4, $5) RETURNING *`,
      [bookId, next.rows[0].next_num, body.title ?? null, content, countWords(content)]
    )

    await client.query('COMMIT')
    return result.rows[0] ?? null
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}

/**
 * Deletes a chapter, renumbers siblings to keep `chapter_number` contiguous,
 * and updates the parent book's word_count. Returns true when deleted.
 */
export async function deleteChapterForUser(
  pool: Pool,
  chapterId: string,
  userId: string
): Promise<{ bookId: string; chapterTitle: string | null } | null> {
  const owned = await findChapterForUser(pool, chapterId, userId)
  if (!owned) return null

  const { booksTable, chaptersTable } = await getContentSchemaTables(pool)
  const bookId = owned.book_id as string
  const chapterTitle = (owned.title as string | null) ?? null

  // One transaction under the shared per-book advisory lock so a concurrent
  // completion can neither observe the deletion mid-renumber nor resurrect it.
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    await acquireBookLock(client, bookId)

    await client.query(`DELETE FROM ${chaptersTable} WHERE id = $1`, [chapterId])

    await client.query(
      `WITH numbered AS (
         SELECT id, ROW_NUMBER() OVER (ORDER BY chapter_number) AS new_num
           FROM ${chaptersTable} WHERE book_id = $1
       )
       UPDATE ${chaptersTable} c SET chapter_number = n.new_num
         FROM numbered n WHERE c.id = n.id`,
      [bookId]
    )

    await client.query(
      `UPDATE ${booksTable}
          SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
              updated_at = NOW()
        WHERE id = $1`,
      [bookId]
    )

    await client.query('COMMIT')
    return { bookId, chapterTitle }
  } catch (error) {
    await client.query('ROLLBACK').catch(() => {})
    throw error
  } finally {
    client.release()
  }
}

/**
 * Best-effort log into `generation_logs`. Failures are swallowed so chapter ops
 * are not blocked by missing/legacy log table installations.
 */
export async function logChapterEvent(
  pool: Pool,
  bookId: string,
  generationType: 'edit' | 'delete' | 'create',
  prompt: string,
  payload: Record<string, unknown>
): Promise<void> {
  try {
    await pool.query(
      `INSERT INTO generation_logs (book_id, generation_type, prompt, status, result)
       VALUES ($1, $2, $3, 'completed', $4)`,
      [bookId, generationType, prompt, JSON.stringify(payload)]
    )
  } catch (error) {
    console.warn('logChapterEvent: skipped due to error:', error)
  }
}
