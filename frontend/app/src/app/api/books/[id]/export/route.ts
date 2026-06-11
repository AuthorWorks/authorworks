import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

const DEFAULT_BOOK_GENERATOR_URL =
  'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'

interface RouteContext {
  params: { id: string }
}

function isSafeComponent(name: string): boolean {
  return /^[A-Za-z0-9][A-Za-z0-9 ._-]*$/.test(name) && !name.includes('..')
}

// GET /api/books/[id]/export            - List downloadable artifacts for a book.
// GET /api/books/[id]/export?file=NAME  - Stream a rendered artifact (epub/pdf/html).
export async function GET(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  const { booksTable, bookOwnerCol } = await getContentSchemaTables(pool)
  const owned = await pool.query(
    `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
    [params.id, userId]
  )
  if (owned.rows.length === 0) {
    return NextResponse.json({ error: 'Book not found' }, { status: 404 })
  }

  const generatorUrl = process.env.BOOK_GENERATOR_URL || DEFAULT_BOOK_GENERATOR_URL
  const file = request.nextUrl.searchParams.get('file')

  try {
    if (!file) {
      const response = await fetch(`${generatorUrl}/api/books/${params.id}/files`, {
        cache: 'no-store',
      })
      if (!response.ok) {
        return NextResponse.json(
          { error: 'No generated artifacts found for this book' },
          { status: response.status === 404 ? 404 : 502 }
        )
      }
      return NextResponse.json(await response.json())
    }

    if (!isSafeComponent(file)) {
      return NextResponse.json({ error: 'Invalid file name' }, { status: 400 })
    }

    const response = await fetch(
      `${generatorUrl}/api/books/${params.id}/files/${encodeURIComponent(file)}`,
      { cache: 'no-store' }
    )
    if (!response.ok) {
      return NextResponse.json({ error: 'File not found' }, { status: response.status })
    }

    return new NextResponse(response.body, {
      status: 200,
      headers: {
        'Content-Type': response.headers.get('content-type') ?? 'application/octet-stream',
        'Content-Disposition':
          response.headers.get('content-disposition') ?? `attachment; filename="${file}"`,
        'Cache-Control': 'no-store',
      },
    })
  } catch (error) {
    console.error('GET /api/books/[id]/export failed:', error)
    return NextResponse.json({ error: 'Book generator service is unreachable' }, { status: 503 })
  }
}
