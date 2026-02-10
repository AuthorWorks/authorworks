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

async function getUserId(request: NextRequest): Promise<string | null> {
  const authHeader = request.headers.get('authorization');
  if (!authHeader?.startsWith('Bearer ')) return null;

  const token = authHeader.substring(7);
  const LOGTO_ENDPOINT =
    process.env.LOGTO_ENDPOINT || 'http://logto.security.svc.cluster.local:3001';

  try {
    const response = await fetch(`${LOGTO_ENDPOINT}/oidc/me`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!response.ok) return null;
    const userInfo = await response.json();
    return userInfo.sub;
  } catch {
    return null;
  }
}

// GET /api/chapters/[id] - Get chapter by ID (with ownership check via book)
export async function GET(request: NextRequest, { params }: { params: { id: string } }) {
  const userId = await getUserId(request);
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
  }

  const pool = getPool();
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool);
    const result = await pool.query(
      `SELECT c.*, b.title as book_title
       FROM ${chaptersTable} c
       JOIN ${booksTable} b ON c.book_id = b.id
       WHERE c.id = $1 AND b.${bookOwnerCol} = $2`,
      [params.id, userId]
    );

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 });
    }

    return NextResponse.json(result.rows[0]);
  } catch (error) {
    console.error('Error fetching chapter:', error);
    return NextResponse.json({ error: 'Failed to fetch chapter' }, { status: 500 });
  } finally {
    await pool.end();
  }
}

// PUT /api/chapters/[id] - Update chapter by ID
export async function PUT(request: NextRequest, { params }: { params: { id: string } }) {
  const userId = await getUserId(request);
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
  }

  const pool = getPool();
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool);
    const ownerCheck = await pool.query(
      `SELECT c.id, c.book_id
       FROM ${chaptersTable} c
       JOIN ${booksTable} b ON c.book_id = b.id
       WHERE c.id = $1 AND b.${bookOwnerCol} = $2`,
      [params.id, userId]
    );

    if (ownerCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 });
    }

    const bookId = ownerCheck.rows[0].book_id;
    const body = await request.json();
    const { title, content } = body;

    const wordCount = content ? content.trim().split(/\s+/).filter(Boolean).length : 0;

    const result = await pool.query(
      `UPDATE ${chaptersTable}
       SET title = COALESCE($1, title),
           content = COALESCE($2, content),
           word_count = $3,
           updated_at = NOW()
       WHERE id = $4
       RETURNING *`,
      [title, content, wordCount, params.id]
    );

    if (result.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 });
    }

    await pool.query(
      `INSERT INTO generation_logs (book_id, generation_type, prompt, status, result)
       VALUES ($1, 'edit', $2, 'completed', $3)`,
      [
        bookId,
        `Chapter "${title}" edited`,
        JSON.stringify({
          chapter_id: params.id,
          word_count: wordCount,
          user_id: userId,
          timestamp: new Date().toISOString(),
        }),
      ]
    );

    await pool.query(
      `UPDATE ${booksTable}
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
           updated_at = NOW()
       WHERE id = $1`,
      [bookId]
    );

    return NextResponse.json(result.rows[0]);
  } catch (error) {
    console.error('Error updating chapter:', error);
    return NextResponse.json({ error: 'Failed to update chapter' }, { status: 500 });
  } finally {
    await pool.end();
  }
}

// DELETE /api/chapters/[id] - Delete chapter by ID
export async function DELETE(request: NextRequest, { params }: { params: { id: string } }) {
  const userId = await getUserId(request);
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
  }

  const pool = getPool();
  try {
    const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool);
    const ownerCheck = await pool.query(
      `SELECT c.id, c.book_id, c.title
       FROM ${chaptersTable} c
       JOIN ${booksTable} b ON c.book_id = b.id
       WHERE c.id = $1 AND b.${bookOwnerCol} = $2`,
      [params.id, userId]
    );

    if (ownerCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 });
    }

    const { book_id: bookId, title: chapterTitle } = ownerCheck.rows[0];

    await pool.query(`DELETE FROM ${chaptersTable} WHERE id = $1`, [params.id]);

    await pool.query(
      `INSERT INTO generation_logs (book_id, generation_type, prompt, status, result)
       VALUES ($1, 'delete', $2, 'completed', $3)`,
      [
        bookId,
        `Chapter "${chapterTitle}" deleted`,
        JSON.stringify({
          chapter_id: params.id,
          user_id: userId,
          timestamp: new Date().toISOString(),
        }),
      ]
    );

    await pool.query(
      `WITH numbered AS (
        SELECT id, ROW_NUMBER() OVER (ORDER BY chapter_number) as new_num
        FROM ${chaptersTable} WHERE book_id = $1
      )
      UPDATE ${chaptersTable} c SET chapter_number = n.new_num
      FROM numbered n WHERE c.id = n.id`,
      [bookId]
    );

    await pool.query(
      `UPDATE ${booksTable}
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1),
           updated_at = NOW()
       WHERE id = $1`,
      [bookId]
    );

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error('Error deleting chapter:', error);
    return NextResponse.json({ error: 'Failed to delete chapter' }, { status: 500 });
  } finally {
    await pool.end();
  }
}
