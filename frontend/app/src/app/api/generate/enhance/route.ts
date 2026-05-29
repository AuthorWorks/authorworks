import { NextRequest, NextResponse } from 'next/server'
import { AiError, chatCompletion } from '@/app/lib/ai'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { findChapterForUser } from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'

const ENHANCEMENT_PROMPTS: Record<string, { system: string; user: (text: string) => string }> = {
  improve: {
    system:
      'You are an expert prose editor. Improve the writing while preserving the author\'s voice and intent. Return ONLY the improved prose with no preamble.',
    user: (text) => `Improve this prose:\n\n${text}`,
  },
  expand: {
    system:
      'You are a novelist. Expand the passage with vivid description and richer character moments while keeping the author\'s style. Return ONLY the expanded prose.',
    user: (text) => `Expand this passage:\n\n${text}`,
  },
  shorten: {
    system:
      'You are a prose editor focused on tight, punchy writing. Compress the passage by ~30-50% without losing important details. Return ONLY the shortened prose.',
    user: (text) => `Shorten this passage:\n\n${text}`,
  },
  rephrase: {
    system:
      'You are a writing coach. Rephrase the passage so it reads more naturally while keeping the same meaning. Return ONLY the rephrased prose.',
    user: (text) => `Rephrase this passage:\n\n${text}`,
  },
  continue: {
    system:
      'You are a novelist continuing the user\'s story. Match their voice, tense, and POV. Return ONLY the continuation.',
    user: (text) => `Continue this story naturally:\n\n${text}`,
  },
}

interface EnhanceBody {
  text?: string
  mode?: keyof typeof ENHANCEMENT_PROMPTS
  chapter_id?: string
  max_tokens?: number
}

// POST /api/generate/enhance - Enhances/transforms a chunk of prose using the configured AI provider.
export async function POST(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: EnhanceBody
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const mode = (body.mode ?? 'improve') as keyof typeof ENHANCEMENT_PROMPTS
  const promptPair = ENHANCEMENT_PROMPTS[mode]
  if (!promptPair) {
    return NextResponse.json(
      { error: `Unknown mode '${mode}'. Allowed: ${Object.keys(ENHANCEMENT_PROMPTS).join(', ')}` },
      { status: 400 }
    )
  }

  let text = body.text?.trim()

  // When invoked from a chapter editor we accept a chapter_id and load the content.
  if (!text && body.chapter_id) {
    const chapter = await findChapterForUser(getPool(), body.chapter_id, userId)
    if (!chapter) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }
    text = ((chapter.content as string) ?? '').trim()
  }

  if (!text) {
    return NextResponse.json({ error: 'text or chapter_id is required' }, { status: 400 })
  }

  // Cap input to keep prompt sizes sane and predictable.
  const MAX_INPUT_CHARS = 20_000
  if (text.length > MAX_INPUT_CHARS) {
    text = text.slice(0, MAX_INPUT_CHARS)
  }

  try {
    const completion = await chatCompletion(promptPair.system, promptPair.user(text), {
      maxTokens: body.max_tokens ?? 4000,
      temperature: 0.7,
    })
    return NextResponse.json({
      mode,
      result: completion.text.trim(),
      model: completion.model,
      provider: completion.provider,
      tokens: { input: completion.inputTokens, output: completion.outputTokens },
    })
  } catch (error) {
    if (error instanceof AiError) {
      return NextResponse.json({ error: 'AI generation failed', details: error.message }, { status: error.status })
    }
    console.error('POST /api/generate/enhance failed:', error)
    return NextResponse.json({ error: 'AI generation failed' }, { status: 500 })
  }
}
