import { Pool } from 'pg'

export type ContentSchemaTables = {
  booksTable: string
  chaptersTable: string
  bookOwnerCol: string
}

/** Detect content schema and return table names + owner column for books. */
export async function getContentSchemaTables(
  pool: Pool
): Promise<ContentSchemaTables> {
  const r = await pool.query(
    `SELECT 1 FROM information_schema.tables WHERE table_schema = 'content' AND table_name = 'books' LIMIT 1`
  )
  const hasContent = (r.rows?.length ?? 0) > 0
  return {
    booksTable: hasContent ? 'content.books' : 'books',
    chaptersTable: hasContent ? 'content.chapters' : 'chapters',
    bookOwnerCol: hasContent ? 'author_id' : 'user_id',
  }
}
