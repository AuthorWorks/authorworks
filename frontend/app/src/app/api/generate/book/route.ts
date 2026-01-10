import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'

// Database connection
function getPool() {
  return new Pool({
    connectionString: process.env.DATABASE_URL || 'postgresql://postgres:homelab_postgres_2024@postgres.databases.svc.cluster.local:5432/authorworks',
  })
}

// Helper to get user ID from auth token
async function getUserId(request: NextRequest): Promise<string | null> {
  const authHeader = request.headers.get('authorization')
  if (!authHeader?.startsWith('Bearer ')) return null

  const token = authHeader.substring(7)
  const LOGTO_ENDPOINT = process.env.LOGTO_ENDPOINT || 'http://logto.security.svc.cluster.local:3001'

  try {
    const response = await fetch(`${LOGTO_ENDPOINT}/oidc/me`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!response.ok) return null
    const userInfo = await response.json()
    return userInfo.sub
  } catch {
    return null
  }
}

// POST /api/generate/book - Start full book generation using core engine
export async function POST(request: NextRequest) {
  const BOOK_GENERATOR_URL = process.env.BOOK_GENERATOR_URL || 'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'

  console.log('POST /api/generate/book - starting')
  console.log('BOOK_GENERATOR_URL:', BOOK_GENERATOR_URL)

  const userId = await getUserId(request)
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
  }

  try {
    const body = await request.json()
    const {
      book_id,
      title,
      description,
      braindump,
      genre,
      style,
      characters,
      synopsis,
      outline_prompt,
      chapter_count,
      author_name,
    } = body

    if (!book_id || !title) {
      return NextResponse.json({ error: 'book_id and title are required' }, { status: 400 })
    }

    console.log('POST /api/generate/book - Calling book generator for book:', book_id)

    // Call the book generator service
    const generatorResponse = await fetch(`${BOOK_GENERATOR_URL}/api/generate`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        book_id,
        title,
        description: description || '',
        braindump: braindump || '',
        genre: genre || '',
        style: style || '',
        characters: characters || '',
        synopsis: synopsis || '',
        outline_prompt: outline_prompt || '',
        chapter_count: chapter_count || 12,
        author_name: author_name || 'AuthorWorks User',
      }),
    })

    if (!generatorResponse.ok) {
      const errorText = await generatorResponse.text()
      console.error('Book generator error:', errorText)
      return NextResponse.json({ error: 'Failed to start book generation' }, { status: 500 })
    }

    const result = await generatorResponse.json()
    console.log('POST /api/generate/book - Job started:', result.job_id)

    // Store the job ID in the database using existing schema
    const pool = getPool()
    try {
      await pool.query(
        `INSERT INTO generation_logs (book_id, generation_type, prompt, status, result)
         VALUES ($1, 'full_book', $2, 'pending', $3)`,
        [
          book_id,
          `Title: ${title}\nDescription: ${description || ''}\nGenre: ${genre || ''}\nStyle: ${style || ''}\nOutline Prompt: ${outline_prompt || ''}`,
          JSON.stringify({ job_id: result.job_id, user_id: userId })
        ]
      )
    } catch (dbError) {
      console.error('Failed to log generation job:', dbError)
      // Don't fail the request, generation still started
    } finally {
      await pool.end()
    }

    return NextResponse.json({
      job_id: result.job_id,
      status: 'started',
      message: 'Book generation started. Poll /api/generate/book/status/:job_id for updates.',
    }, { status: 202 })
  } catch (error) {
    console.error('POST /api/generate/book - Error:', error)
    return NextResponse.json({ error: 'Internal server error' }, { status: 500 })
  }
}
