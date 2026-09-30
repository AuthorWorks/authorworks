import type { PoolClient } from 'pg'

const LOCK_PREFIX = 'aw:book:'

/**
 * Acquires the per-book advisory lock shared by author chapter mutations
 * (create/edit/delete) and generation completion. Taking the same lock on both
 * paths means a completion can never interleave with an author save, and a
 * save can never land between a completion's snapshot and its writes.
 * Transaction-scoped: released automatically at COMMIT/ROLLBACK.
 */
export async function acquireBookLock(
  client: PoolClient,
  bookId: string
): Promise<void> {
  await client.query(`SELECT pg_advisory_xact_lock(hashtextextended($1, 0))`, [
    LOCK_PREFIX + bookId,
  ])
}
