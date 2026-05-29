import { NextRequest, NextResponse } from 'next/server'
import { AiError, chatCompletion, extractJson } from '@/app/lib/ai'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { countWords } from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'
import { getContentSchemaTables } from '@/app/lib/db-schema'

interface OutlineRequest {
  book_id: string
  prompt: string
  genre?: string
  style?: string
  chapter_count?: number
}

interface GeneratedChapter {
  title: string
  summary: string
  key_events?: string[]
}

interface GeneratedOutline {
  synopsis: string
  themes: string[]
  chapters: GeneratedChapter[]
}

const SYSTEM_PROMPT = `You are a professional author and book outliner. Create detailed, compelling book outlines that provide a strong foundation for novel writing.

Output ONLY valid JSON with this structure:
{
  "synopsis": "2-3 paragraph synopsis of the entire story",
  "themes": ["theme1", "theme2", "theme3"],
  "chapters": [
    { "title": "Chapter 1 Title", "summary": "2-3 sentence summary of what happens", "key_events": ["event1", "event2"] }
  ]
}`

// POST /api/generate/outline - Generate AI outline and seed chapters for a book.
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: OutlineRequest
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const { book_id, prompt, genre, style, chapter_count = 12 } = body
  if (!book_id) {
    return NextResponse.json({ error: 'book_id is required' }, { status: 400 })
  }

  const pool = getPool()
  const { booksTable, chaptersTable, bookOwnerCol } = await getContentSchemaTables(pool)

  const bookResult = await pool.query(
    `SELECT id, title, description, metadata FROM ${booksTable} WHERE id = $1 AND ${bookOwnerCol} = $2`,
    [book_id, userId]
  )
  if (bookResult.rows.length === 0) {
    return NextResponse.json({ error: 'Book not found' }, { status: 404 })
  }
  const book = bookResult.rows[0]
  const metadata = book.metadata || {}

  const userPrompt = `Create a ${chapter_count}-chapter outline for a ${genre || 'fiction'} novel.

Title: ${book.title}
Description: ${book.description || 'Not provided'}
${style ? `Writing Style: ${style}` : ''}
${metadata.braindump ? `Creative Ideas: ${metadata.braindump}` : ''}
${metadata.characters ? `Characters: ${metadata.characters}` : ''}
${metadata.synopsis ? `Story Synopsis: ${metadata.synopsis}` : ''}
${prompt ? `Additional Direction: ${prompt}` : ''}

Generate a compelling, well-paced outline with ${chapter_count} chapters. Each chapter should advance the plot meaningfully.`

  const logResult = await pool.query(
    `INSERT INTO generation_logs (book_id, generation_type, prompt, model, status)
     VALUES ($1, 'outline', $2, $3, 'processing') RETURNING id`,
    [book_id, userPrompt, 'pending']
  )
  const logId = logResult.rows[0].id

  let completion
  try {
    completion = await chatCompletion(SYSTEM_PROMPT, userPrompt, { maxTokens: 8000 })
  } catch (error) {
    const message = error instanceof AiError ? error.message : String(error)
    await pool.query(
      `UPDATE generation_logs SET status = 'failed', error = $1, completed_at = NOW() WHERE id = $2`,
      [message, logId]
    )
    const status = error instanceof AiError ? error.status : 500
    return NextResponse.json({ error: 'AI generation failed' }, { status })
  }

  let outline: GeneratedOutline
  try {
    outline = extractJson<GeneratedOutline>(completion.text)
  } catch {
    await pool.query(
      `UPDATE generation_logs SET status = 'failed', error = $1, result = $2,
         input_tokens = $3, output_tokens = $4, completed_at = NOW() WHERE id = $5`,
      [
        'Failed to parse AI response',
        JSON.stringify({ raw_response: completion.text }),
        completion.inputTokens,
        completion.outputTokens,
        logId,
      ]
    )
    return NextResponse.json({ error: 'Failed to parse AI response' }, { status: 502 })
  }

  for (let i = 0; i < outline.chapters.length; i++) {
    const chapter = outline.chapters[i]
    const content = [
      `## ${chapter.title}`,
      '',
      chapter.summary,
      '',
      chapter.key_events?.length
        ? '### Key Events\n' + chapter.key_events.map((e) => `- ${e}`).join('\n')
        : '',
    ]
      .filter(Boolean)
      .join('\n')

    await pool.query(
      `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count)
       VALUES ($1, $2, $3, $4, $5)`,
      [book_id, i + 1, chapter.title, content, countWords(content)]
    )
  }

  await pool.query(
    `UPDATE ${booksTable} SET
       metadata = COALESCE(metadata, '{}'::jsonb) || $1,
       updated_at = NOW(),
       word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $2)
     WHERE id = $2`,
    [
      JSON.stringify({
        outline_generated: true,
        ai_synopsis: outline.synopsis,
        themes: outline.themes,
      }),
      book_id,
    ]
  )

  await pool.query(
    `UPDATE generation_logs SET
       status = 'completed', model = $1, result = $2,
       input_tokens = $3, output_tokens = $4, completed_at = NOW()
     WHERE id = $5`,
    [completion.model, JSON.stringify(outline), completion.inputTokens, completion.outputTokens, logId]
  )

  return NextResponse.json({
    success: true,
    synopsis: outline.synopsis,
    themes: outline.themes,
    chapters_created: outline.chapters.length,
    tokens: { input: completion.inputTokens, output: completion.outputTokens },
  })
}
