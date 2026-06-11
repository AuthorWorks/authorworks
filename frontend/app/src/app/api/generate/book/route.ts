import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'
import { getUserAiOverride } from '@/app/lib/user-ai'

const DEFAULT_BOOK_GENERATOR_URL =
  'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'

// POST /api/generate/book - Kick off full book generation in the book-generator service.
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: {
    book_id?: string
    title?: string
    description?: string
    braindump?: string
    genre?: string
    style?: string
    characters?: string
    synopsis?: string
    outline_prompt?: string
    chapter_count?: number
    author_name?: string
  }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  if (!body.book_id || !body.title) {
    return NextResponse.json({ error: 'book_id and title are required' }, { status: 400 })
  }

  // Only the book's owner may trigger (re)generation.
  const { booksTable, bookOwnerCol } = await getContentSchemaTables(getPool())
  const owned = await getPool().query(
    `SELECT id FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
    [body.book_id, userId]
  )
  if (owned.rows.length === 0) {
    return NextResponse.json({ error: 'Book not found' }, { status: 404 })
  }

  const generatorUrl = process.env.BOOK_GENERATOR_URL || DEFAULT_BOOK_GENERATOR_URL

  // Bring-your-own inference: forward the user's OpenAI-compatible endpoint to
  // the generator. Null override means the platform default (internal LiteLLM).
  const override = await getUserAiOverride(getPool(), userId)

  let generatorResponse: Response
  try {
    generatorResponse = await fetch(`${generatorUrl}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        book_id: body.book_id,
        title: body.title,
        description: body.description ?? '',
        braindump: body.braindump ?? '',
        genre: body.genre ?? '',
        style: body.style ?? '',
        characters: body.characters ?? '',
        synopsis: body.synopsis ?? '',
        outline_prompt: body.outline_prompt ?? '',
        chapter_count: body.chapter_count ?? 12,
        author_name: body.author_name ?? 'AuthorWorks User',
        ...(override
          ? {
              llm_api_base: override.baseUrl,
              llm_model: override.model,
              llm_api_key: override.apiKey ?? '',
            }
          : {}),
      }),
    })
  } catch (error) {
    console.error('Book generator unreachable:', error)
    return NextResponse.json(
      { error: 'Book generator service is unreachable' },
      { status: 503 }
    )
  }

  if (!generatorResponse.ok) {
    const errorText = await generatorResponse.text()
    return NextResponse.json(
      { error: 'Failed to start book generation', details: errorText },
      { status: generatorResponse.status }
    )
  }

  const result = await generatorResponse.json()

  // Record the active job on the book so the UI can resume progress polling
  // after a reload (the status route flips this to completed/failed).
  try {
    await getPool().query(
      `UPDATE ${booksTable} SET
         metadata = COALESCE(metadata, '{}'::jsonb) || $1,
         updated_at = NOW()
       WHERE id = $2`,
      [
        JSON.stringify({
          generation_job_id: result.job_id,
          generation_status: 'running',
          generation_started_at: new Date().toISOString(),
        }),
        body.book_id,
      ]
    )
  } catch (error) {
    console.warn('Failed to record generation job on book (non-fatal):', error)
  }

  // Best-effort logging - don't fail the request if logging breaks.
  try {
    await getPool().query(
      `INSERT INTO generation_logs (book_id, generation_type, prompt, status, result)
       VALUES ($1, 'full_book', $2, 'pending', $3)`,
      [
        body.book_id,
        `Title: ${body.title}\nDescription: ${body.description || ''}\nGenre: ${body.genre || ''}\nStyle: ${body.style || ''}\nOutline Prompt: ${body.outline_prompt || ''}`,
        JSON.stringify({ job_id: result.job_id, user_id: userId }),
      ]
    )
  } catch (error) {
    console.warn('Failed to log generation job (non-fatal):', error)
  }

  return NextResponse.json(
    {
      job_id: result.job_id,
      status: 'started',
      message: 'Book generation started. Poll /api/generate/book/status/:job_id for updates.',
    },
    { status: 202 }
  )
}
