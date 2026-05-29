import type { Pool } from 'pg'
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

/** Updates a chapter's title/content and returns the new row, or null when not owned. */
export async function updateChapterForUser(
  pool: Pool,
  chapterId: string,
  userId: string,
  patch: { title?: string | null; content?: string | null }
): Promise<Record<string, unknown> | null> {
  const owned = await findChapterForUser(pool, chapterId, userId)
  if (!owned) return null

  const { booksTable, chaptersTable } = await getContentSchemaTables(pool)
  const wordCount = countWords(patch.content as string | null | undefined)
  const result = await pool.query(
    `UPDATE ${chaptersTable}
        SET title = COALESCE($1, title),
            content = COALESCE($2, content),
            word_count = $3,
            updated_at = NOW()
      WHERE id = $4
      RETURNING *`,
    [patch.title ?? null, patch.content ?? null, wordCount, chapterId]
  )

  await pool.query(
    `UPDATE ${booksTable}
        SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
            updated_at = NOW()
      WHERE id = $1`,
    [owned.book_id]
  )

  return result.rows[0] ?? null
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

  await pool.query(`DELETE FROM ${chaptersTable} WHERE id = $1`, [chapterId])

  await pool.query(
    `WITH numbered AS (
       SELECT id, ROW_NUMBER() OVER (ORDER BY chapter_number) AS new_num
         FROM ${chaptersTable} WHERE book_id = $1
     )
     UPDATE ${chaptersTable} c SET chapter_number = n.new_num
       FROM numbered n WHERE c.id = n.id`,
    [bookId]
  )

  await pool.query(
    `UPDATE ${booksTable}
        SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
            updated_at = NOW()
      WHERE id = $1`,
    [bookId]
  )

  return { bookId, chapterTitle }
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
