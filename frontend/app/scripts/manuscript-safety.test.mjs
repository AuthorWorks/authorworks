/**
 * Manuscript-safety integration tests — node:test against a REAL, isolated
 * PostgreSQL instance (see README notes in this directory).
 *
 * Required env:
 *   MANUSCRIPT_TEST_DB  postgres connection string, e.g.
 *                       postgres://127.0.0.1:55432/manuscript_safety_test
 *   MANUSCRIPT_LIB_DIR  compiled CJS output of src/app/lib/{manuscript,chapters,book-lock,db-schema}.ts
 *
 * These tests exercise the SHARED completion implementation used by BOTH the
 * status-polling auto-sync and the explicit /api/generate/book/sync route.
 */
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import pg from 'pg'

const here = path.dirname(fileURLToPath(import.meta.url))
const LIB = process.env.MANUSCRIPT_LIB_DIR
if (!LIB) {
  console.error('Set MANUSCRIPT_LIB_DIR to the compiled lib directory')
  process.exit(1)
}
const dbUrl = process.env.MANUSCRIPT_TEST_DB

const createRequire = (await import('node:module')).createRequire
const require = createRequire(import.meta.url)
const manuscript = require(path.join(LIB, 'manuscript.js'))
const chaptersLib = require(path.join(LIB, 'chapters.js'))

const pool = new pg.Pool({ connectionString: dbUrl, max: 10 })

const USER_A = 'user-a-logto-sub'
const USER_B = 'user-b-logto-sub'

async function resetDb() {
  await pool.query('TRUNCATE generation_completions, generation_logs, chapters, books')
}

async function makeBook(userId, meta = {}, chapterSeed = 2) {
  const bookId = crypto.randomUUID()
  await pool.query(
    `INSERT INTO books (id, user_id, title, metadata) VALUES ($1, $2, 'Test Book', $3::jsonb)`,
    [bookId, userId, JSON.stringify(meta)]
  )
  const ids = []
  for (let i = 1; i <= chapterSeed; i++) {
    ids.push(await makeChapter(bookId, i, `Chapter ${i}`, `Original content ${i}.`))
  }
  return { bookId, chapterIds: ids }
}

async function makeChapter(bookId, number, title, content) {
  const id = crypto.randomUUID()
  await pool.query(
    `INSERT INTO chapters (id, book_id, chapter_number, title, content, word_count)
     VALUES ($1, $2, $3, $4, $5, $6)`,
    [id, bookId, number, title, content, content.trim().split(/\s+/).length]
  )
  return id
}

async function chaptersOf(bookId) {
  return (
    await pool.query(
      `SELECT id, chapter_number, title, content, word_count, updated_at
         FROM chapters WHERE book_id = $1 ORDER BY chapter_number`,
      [bookId]
    )
  ).rows
}

async function bookMeta(bookId) {
  const r = await pool.query(`SELECT metadata, word_count, status FROM books WHERE id = $1`, [bookId])
  return r.rows[0]
}

function genOutput(titles) {
  return {
    chapters: titles.map((t, i) => ({ title: t, content: `Generated ${t} body ${i + 1}.` })),
    synopsis: 'A generated synopsis',
    themes: ['regeneration'],
  }
}

// ---------------------------------------------------------------------------

test('setup: fixture schema + migration applied', async () => {
  // Applied by the runner script before tests; assert the live-like shape.
  const ch = await pool.query(
    `SELECT column_name FROM information_schema.columns WHERE table_name='chapters'`
  )
  const cols = ch.rows.map((r) => r.column_name)
  assert.ok(!cols.includes('metadata'), 'live chapters has no metadata column')
  const uq = await pool.query(
    `SELECT 1 FROM pg_constraint WHERE conrelid='chapters'::regclass AND contype='u'`
  )
  assert.equal(uq.rowCount, 0, 'no unique chapter_number constraint, like live')
})

test('safe apply: baseline unchanged -> applies, preserves chapter IDs', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await pool.query(
    `INSERT INTO generation_logs (book_id, generation_type, status, result)
     VALUES ($1, 'full_book', 'pending', $2::jsonb)`,
    [bookId, JSON.stringify({ job_id: 'job-safe-1', user_id: USER_A })]
  )

  await manuscript.recordGenerationStart(pool, bookId, 'job-safe-1', {
    generation_job_id: 'job-safe-1',
    generation_status: 'running',
  })

  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId,
    jobId: 'job-safe-1',
    userId: USER_A,
    output: genOutput(['New One', 'New Two', 'New Three']),
  })
  assert.equal(result.outcome, 'applied')

  const rows = await chaptersOf(bookId)
  assert.equal(rows.length, 3)
  // Existing chapters keep their stable ids; only the new 3rd chapter gets one.
  assert.deepEqual(
    rows.slice(0, chapterIds.length).map((r) => r.id),
    chapterIds,
    'chapter IDs must be preserved'
  )
  assert.equal(rows[0].title, 'New One')
  assert.ok(rows[0].content.includes('Generated New One body'))
  assert.equal(rows[0].word_count, 5)

  const meta = await bookMeta(bookId)
  assert.equal(meta.metadata.generation_status, 'completed')
  assert.equal(meta.metadata.generation_completed, true)
  assert.equal(meta.metadata.ai_synopsis, 'A generated synopsis')
  assert.equal(meta.word_count, 15)

  const log = await pool.query(`SELECT status FROM generation_logs WHERE book_id=$1`, [bookId])
  assert.equal(log.rows[0].status, 'completed')

  const record = await pool.query(
    `SELECT status, book_id, user_id FROM generation_completions WHERE job_id=$1`,
    ['job-safe-1']
  )
  assert.equal(record.rows[0].status, 'applied')
  assert.equal(record.rows[0].user_id, USER_A)
})

test('duplicate completion is idempotent (sequential)', async () => {
  await resetDb()
  const { bookId } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-dup', {})
  const output = genOutput(['A', 'B'])
  const first = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-dup', userId: USER_A, output,
  })
  assert.equal(first.outcome, 'applied')
  const after = await chaptersOf(bookId)

  const second = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-dup', userId: USER_A, output,
  })
  assert.equal(second.outcome, 'already_applied')
  assert.deepEqual(await chaptersOf(bookId), after, 'no second mutation')
  const n = await pool.query(`SELECT count(*) AS c FROM generation_completions`)
  assert.equal(Number(n.rows[0].c), 1)
})

test('duplicate completion is idempotent (concurrent)', async () => {
  await resetDb()
  const { bookId } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-conc', {})
  const output = genOutput(['X', 'Y'])
  const [r1, r2] = await Promise.all([
    manuscript.applyGenerationCompletion(pool, { bookId, jobId: 'job-conc', userId: USER_A, output }),
    manuscript.applyGenerationCompletion(pool, { bookId, jobId: 'job-conc', userId: USER_A, output }),
  ])
  const outcomes = [r1.outcome, r2.outcome].sort()
  assert.deepEqual(outcomes, ['already_applied', 'applied'])
  const rows = await chaptersOf(bookId)
  assert.equal(rows.length, 2, 'exactly one apply happened')
})

test('author edit after generation start -> pending_author_approval, edits preserved', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-edit', {})

  // Author edits a chapter while the job runs.
  const updated = await chaptersLib.updateChapterForUser(
    pool, chapterIds[0], USER_A, { content: 'Precious author text. Kept forever.' }
  )
  assert.ok(updated && updated !== 'conflict')

  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-edit', userId: USER_A, output: genOutput(['Gen 1', 'Gen 2']),
  })
  assert.equal(result.outcome, 'pending_author_approval')
  assert.equal(result.reason, 'author_edits_since_start')

  const rows = await chaptersOf(bookId)
  assert.equal(rows.length, 2)
  assert.equal(rows[0].id, chapterIds[0])
  assert.equal(rows[0].content, 'Precious author text. Kept forever.')
  assert.equal(rows[0].title, 'Chapter 1')

  const meta = (await bookMeta(bookId)).metadata
  assert.equal(meta.generation_status, 'pending_approval')
  const pending = meta.generation_pending
  assert.equal(pending.job_id, 'job-edit')
  assert.equal(pending.chapters.length, 2)
  assert.ok(pending.chapters[0].content.includes('Generated Gen 1'))

  // Author explicitly accepts the proposal -> applies over current text.
  const accept = await manuscript.resolvePendingProposal(pool, {
    bookId, jobId: 'job-edit', userId: USER_A, accept: true,
  })
  assert.deepEqual(accept, { ok: true, outcome: 'accepted' })
  const applied = await chaptersOf(bookId)
  assert.equal(applied[0].id, chapterIds[0])
  assert.ok(applied[0].content.includes('Generated Gen 1'))
  assert.equal((await bookMeta(bookId)).metadata.generation_status, 'completed')
  // Accepting twice is rejected.
  const again = await manuscript.resolvePendingProposal(pool, {
    bookId, jobId: 'job-edit', userId: USER_A, accept: true,
  })
  assert.equal(again.outcome, 'not_pending')
})

test('author-created chapter between jobs -> pending, not clobbered', async () => {
  await resetDb()
  const { bookId } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-new', {})
  const created = await chaptersLib.createChapterForUser(pool, bookId, USER_A, {
    title: 'My own chapter', content: 'Hand written.',
  })
  assert.ok(created)
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-new', userId: USER_A, output: genOutput(['A', 'B']),
  })
  assert.equal(result.outcome, 'pending_author_approval')
  const rows = await chaptersOf(bookId)
  assert.equal(rows.length, 3)
  assert.ok(rows.some((r) => r.id === created.id && r.title === 'My own chapter'))
})

test('decline keeps the current manuscript', async () => {
  await resetDb()
  const { bookId } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-dec', {})
  await chaptersLib.updateChapterForUser(pool, (await chaptersOf(bookId))[0].id, USER_A, {
    content: 'Keep me.',
  })
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-dec', userId: USER_A, output: genOutput(['A', 'B']),
  })
  assert.equal(result.outcome, 'pending_author_approval')
  const decline = await manuscript.resolvePendingProposal(pool, {
    bookId, jobId: 'job-dec', userId: USER_A, accept: false,
  })
  assert.deepEqual(decline, { ok: true, outcome: 'declined' })
  const rows = await chaptersOf(bookId)
  assert.equal(rows[0].content, 'Keep me.')
  const meta = (await bookMeta(bookId)).metadata
  assert.equal(meta.generation_pending, undefined)
})

test('legacy job (active job id, no baseline) -> pending, chapters preserved', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A, {
    generation_job_id: 'job-legacy',
    generation_status: 'running',
  })
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-legacy', userId: USER_A, output: genOutput(['L1', 'L2']),
  })
  assert.equal(result.outcome, 'pending_author_approval')
  assert.equal(result.reason, 'legacy_no_baseline')
  const rows = await chaptersOf(bookId)
  assert.deepEqual(rows.map((r) => r.id), chapterIds)
  assert.equal(rows[0].content, 'Original content 1.')
})

test('historical replay of completed book -> rejected_stale', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A, {
    generation_completed: true,
    generation_job_id: 'job-old',
    // production's historical book has no generation_status
  })
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-old', userId: USER_A, output: genOutput(['H1', 'H2']),
  })
  assert.equal(result.outcome, 'rejected_stale')
  const rows = await chaptersOf(bookId)
  assert.deepEqual(rows.map((r) => r.id), chapterIds)
  assert.equal(rows[0].content, 'Original content 1.')
  const record = await pool.query(
    `SELECT status FROM generation_completions WHERE job_id=$1`, ['job-old']
  )
  assert.equal(record.rows[0].status, 'rejected_stale')
})

test('stale job (newer active job) -> rejected_stale, nothing changes', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-v2', {})
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-v1', userId: USER_A, output: genOutput(['S1', 'S2']),
  })
  assert.equal(result.outcome, 'rejected_stale')
  const rows = await chaptersOf(bookId)
  assert.deepEqual(rows.map((r) => r.id), chapterIds)
  assert.equal(rows[0].content, 'Original content 1.')
})

test('unauthorized user -> not_found, nothing mutated', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-auth', {})
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-auth', userId: USER_B, output: genOutput(['A', 'B']),
  })
  assert.equal(result.outcome, 'not_found')
  assert.equal((await chaptersOf(bookId)).length, 2)
  const n = await pool.query(`SELECT count(*) AS c FROM generation_completions`)
  assert.equal(Number(n.rows[0].c), 0, 'no completion record for a foreign user')
})

test('job bound to another book/user -> rejected_bound', async () => {
  await resetDb()
  const bookA = await makeBook(USER_A)
  const bookB = await makeBook(USER_B)
  // job-bound is already durably recorded against book B / user B
  await pool.query(
    `INSERT INTO generation_completions (job_id, book_id, user_id, status)
     VALUES ('job-bound', $1, $2, 'applied')`,
    [bookB.bookId, USER_B]
  )
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId: bookA.bookId, jobId: 'job-bound', userId: USER_A, output: genOutput(['A', 'B']),
  })
  assert.equal(result.outcome, 'rejected_bound')
  const rows = await chaptersOf(bookA.bookId)
  assert.equal(rows.length, 2)
  assert.equal(rows[0].content, 'Original content 1.')
})

test('partial-write failure rolls back completely; retry succeeds', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-boom', {})

  // Make the second chapter write explode mid-transaction.
  await pool.query(`DROP TRIGGER IF EXISTS boom_trg ON chapters`)
  await pool.query(
    `CREATE OR REPLACE FUNCTION boom() RETURNS trigger AS $fn$
     BEGIN
       IF NEW.title = 'BOOM' THEN
         RAISE EXCEPTION 'injected mid-apply failure';
       END IF;
       RETURN NEW;
     END $fn$ LANGUAGE plpgsql`
  )
  await pool.query(
    `CREATE TRIGGER boom_trg BEFORE INSERT OR UPDATE ON chapters FOR EACH ROW EXECUTE FUNCTION boom()`
  )

  await assert.rejects(
    manuscript.applyGenerationCompletion(pool, {
      bookId, jobId: 'job-boom', userId: USER_A,
      output: genOutput(['Fine One', 'BOOM', 'Fine Three']),
    }),
    /injected mid-apply failure/
  )

  // Nothing from the failed attempt survived.
  const rows = await chaptersOf(bookId)
  assert.deepEqual(rows.map((r) => r.id), chapterIds)
  assert.equal(rows[0].content, 'Original content 1.')
  const record = await pool.query(
    `SELECT * FROM generation_completions WHERE job_id='job-boom'`
  )
  assert.equal(record.rowCount, 0, 'claim row must roll back with the apply')
  const meta = (await bookMeta(bookId)).metadata
  assert.notEqual(meta.generation_status, 'completed')

  // Retry with a healthy output succeeds and applies cleanly.
  const retry = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-boom', userId: USER_A, output: genOutput(['Retry One', 'Retry Two']),
  })
  assert.equal(retry.outcome, 'applied')
  const after = await chaptersOf(bookId)
  assert.deepEqual(after.map((r) => r.id), chapterIds)
  assert.equal(after[0].title, 'Retry One')
})

test('completion serializes against an in-flight author save (advisory lock)', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-lock', {})

  // Simulate an author save holding the book lock mid-transaction.
  const holder = await pool.connect()
  await holder.query('BEGIN')
  await holder.query(`SELECT pg_advisory_xact_lock(hashtextextended($1, 0))`, [
    'aw:book:' + bookId,
  ])

  const blocked = new pg.Pool({ connectionString: dbUrl, max: 2 })
  let attempt
  try {
    attempt = manuscript.applyGenerationCompletion(
      { connect: () => blocked.connect(), query: blocked.query.bind(blocked) },
      { bookId, jobId: 'job-lock', userId: USER_A, output: genOutput(['L1', 'L2']) }
    )
    const timeout = new Promise((_, rej) => setTimeout(() => rej(new Error('test-timeout')), 2000))
    await assert.rejects(
      Promise.race([attempt, timeout]),
      (err) => err.message === 'test-timeout',
      'completion must block while the author save holds the lock'
    )
    // Author save finishes (roll back its tx) -> the queued completion proceeds.
    await holder.query('ROLLBACK')
    holder.release()
    const queued = await attempt
    assert.equal(queued.outcome, 'applied')
  } finally {
    await blocked.end()
  }

  // A retry of the same job is a no-op.
  const retry = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-lock', userId: USER_A, output: genOutput(['L1', 'L2']),
  })
  assert.equal(retry.outcome, 'already_applied')
})

test('optimistic concurrency: stale expected_updated_at -> conflict', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  const row = (await chaptersOf(bookId))[0]

  const ok = await chaptersLib.updateChapterForUser(pool, chapterIds[0], USER_A, {
    content: 'first save',
  }, { expected_updated_at: row.updated_at })
  assert.ok(ok && ok !== 'conflict')

  const stale = await chaptersLib.updateChapterForUser(pool, chapterIds[0], USER_A, {
    content: 'second save with stale expectation',
  }, { expected_updated_at: row.updated_at })
  assert.equal(stale, 'conflict', 'second save must not overwrite the first')

  const rows = await chaptersOf(bookId)
  assert.equal(rows[0].content, 'first save')
})

test('delete chapter updates the baseline state; next completion goes pending', async () => {
  await resetDb()
  const { bookId, chapterIds } = await makeBook(USER_A)
  await manuscript.recordGenerationStart(pool, bookId, 'job-del', {})
  await chaptersLib.deleteChapterForUser(pool, chapterIds[1], USER_A)
  const result = await manuscript.applyGenerationCompletion(pool, {
    bookId, jobId: 'job-del', userId: USER_A, output: genOutput(['A', 'B']),
  })
  assert.equal(result.outcome, 'pending_author_approval')
  assert.equal((await chaptersOf(bookId)).length, 1)
})

test.after(async () => {
  await pool.end()
})
