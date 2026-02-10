import { NextRequest, NextResponse } from 'next/server';
import { Pool } from 'pg';
import { getContentSchemaTables } from '@/app/lib/db-schema';

function getPool() {
  return new Pool({
    connectionString:
      process.env.DATABASE_URL ||
      'postgresql://postgres:homelab_postgres_2024@postgres.databases.svc.cluster.local:5432/authorworks',
  });
}

// GET /api/generate/book/status/:jobId - Get job status from book generator
export async function GET(request: NextRequest, { params }: { params: { jobId: string } }) {
  const BOOK_GENERATOR_URL =
    process.env.BOOK_GENERATOR_URL ||
    'http://authorworks-book-generator.authorworks.svc.cluster.local:8081';
  const { jobId } = params;

  console.log('GET /api/generate/book/status - jobId:', jobId);

  try {
    const response = await fetch(`${BOOK_GENERATOR_URL}/api/jobs/${jobId}`, {
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      if (response.status === 404) {
        return NextResponse.json({ error: 'Job not found' }, { status: 404 });
      }
      return NextResponse.json({ error: 'Failed to get job status' }, { status: 500 });
    }

    const status = await response.json();

    // Auto-sync when job is completed
    if (status.status === 'completed' && status.book_id && !status.synced) {
      console.log(`Job ${jobId} completed, auto-syncing book ${status.book_id}`);
      await autoSyncBook(status.book_id, jobId, status);
      status.synced = true;
    }

    return NextResponse.json(status);
  } catch (error) {
    console.error('GET /api/generate/book/status - Error:', error);
    return NextResponse.json({ error: 'Internal server error' }, { status: 500 });
  }
}

// Auto-sync completed book to database
async function autoSyncBook(bookId: string, jobId: string, status: any) {
  const pool = getPool();
  try {
    const { booksTable, chaptersTable } = await getContentSchemaTables(pool);
    const chapters = status.chapters || status.outline?.chapters || [];

    if (chapters.length > 0) {
      await pool.query(`DELETE FROM ${chaptersTable} WHERE book_id = $1`, [bookId]);

      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i];
        const content = chapter.content || chapter.summary || '';
        const title = chapter.title || `Chapter ${i + 1}`;

        await pool.query(
          `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count, status)
           VALUES ($1, $2, $3, $4, $5, 'draft')`,
          [bookId, i + 1, title, content, content.split(/\s+/).length]
        );
      }
      console.log(`Synced ${chapters.length} chapters for book ${bookId}`);
    }

    const metadata: Record<string, any> = {
      generation_completed: true,
      generation_job_id: jobId,
      completed_at: new Date().toISOString(),
    };
    if (status.synopsis) metadata.ai_synopsis = status.synopsis;
    if (status.themes) metadata.themes = status.themes;

    await pool.query(
      `UPDATE ${booksTable} SET
         metadata = COALESCE(metadata, '{}'::jsonb) || $1,
         status = 'draft',
         updated_at = NOW()
       WHERE id = $2`,
      [JSON.stringify(metadata), bookId]
    );

    await pool.query(
      `UPDATE ${booksTable}
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1)
       WHERE id = $1`,
      [bookId]
    );

    // Update generation log
    await pool.query(
      `UPDATE generation_logs
       SET status = 'completed', completed_at = NOW()
       WHERE book_id = $1 AND result->>'job_id' = $2`,
      [bookId, jobId]
    );

    console.log(`Book ${bookId} synced successfully`);
  } catch (error) {
    console.error(`Failed to auto-sync book ${bookId}:`, error);
    // Don't throw - we still want to return the status
  } finally {
    await pool.end();
  }
}
