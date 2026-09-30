import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'
import { resolvePendingProposal } from '@/app/lib/manuscript'

interface RouteContext {
  params: { id: string }
}

/**
 * GET /api/books/[id]/generation-pending - Return the pending generated
 * revision stored for this book (if any). Read-only, owner-scoped.
 */
export async function GET(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
    const result = await pool.query(
      `SELECT metadata->'generation_pending' AS pending
         FROM ${booksTable}
        WHERE id = $1 AND ${bookOwnerCol} = $2`,
      [params.id, userId]
    )
    const row = result.rows[0]
    if (!row) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 })
    }
    return NextResponse.json({ pending: row.pending ?? null })
  } catch (error) {
    console.error('GET /api/books/[id]/generation-pending failed:', error)
    return NextResponse.json(
      { error: 'Failed to fetch pending revision' },
      { status: 500 }
    )
  }
}

/**
 * POST /api/books/[id]/generation-pending - Explicitly accept or decline the
 * stored pending revision. Body: { job_id: string, action: 'accept' | 'decline' }.
 * This is the author's conflict-resolution path; nothing here runs automatically.
 */
export async function POST(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: { job_id?: string; action?: string }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  if (!body.job_id || (body.action !== 'accept' && body.action !== 'decline')) {
    return NextResponse.json(
      { error: 'job_id and action ("accept" or "decline") are required' },
      { status: 400 }
    )
  }

  const pool = getPool()
  try {
    const result = await resolvePendingProposal(pool, {
      bookId: params.id,
      jobId: body.job_id,
      userId,
      accept: body.action === 'accept',
    })

    switch (result.outcome) {
      case 'accepted':
        return NextResponse.json({ success: true, outcome: 'accepted' })
      case 'declined':
        return NextResponse.json({ success: true, outcome: 'declined' })
      case 'not_found':
        return NextResponse.json({ error: 'Book not found' }, { status: 404 })
      case 'not_pending':
        return NextResponse.json(
          { error: 'No pending revision for this job' },
          { status: 409 }
        )
    }
  } catch (error) {
    console.error('POST /api/books/[id]/generation-pending failed:', error)
    return NextResponse.json(
      { error: 'Failed to resolve pending revision' },
      { status: 500 }
    )
  }
}
