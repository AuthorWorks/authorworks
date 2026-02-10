import { NextRequest, NextResponse } from 'next/server';
import { Pool } from 'pg';

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

interface OutlineRequest {
  book_id: string;
  prompt: string;
  genre?: string;
  style?: string;
  chapter_count?: number;
}

interface GeneratedChapter {
  title: string;
  summary: string;
  key_events: string[];
}

// POST /api/generate/outline - Generate AI outline for a book
export async function POST(request: NextRequest) {
  const userId = await getUserId(request);
  if (!userId) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
  }

  const pool = getPool();
  try {
    const body: OutlineRequest = await request.json();
    const { book_id, prompt, genre, style, chapter_count = 12 } = body;

    const hasContentSchema = await pool
      .query(
        `SELECT 1 FROM information_schema.tables WHERE table_schema = 'content' AND table_name = 'books' LIMIT 1`
      )
      .then((r: { rows: unknown[] }) => r.rows.length > 0);

    const bookCheck = hasContentSchema
      ? await pool.query(
          'SELECT id, title, description, metadata FROM content.books WHERE id = $1::uuid AND author_id = $2::uuid',
          [book_id, userId]
        )
      : await pool.query(
          'SELECT id, title, description, metadata FROM books WHERE id = $1 AND user_id = $2',
          [book_id, userId]
        );
    if (bookCheck.rows.length === 0) {
      return NextResponse.json({ error: 'Book not found' }, { status: 404 });
    }

    const book = bookCheck.rows[0];
    const metadata = book.metadata || {};

    // Build comprehensive prompt for AI
    const systemPrompt = `You are a professional author and book outliner. Create detailed, compelling book outlines that provide a strong foundation for novel writing.

Output ONLY valid JSON with this structure:
{
  "synopsis": "2-3 paragraph synopsis of the entire story",
  "themes": ["theme1", "theme2", "theme3"],
  "chapters": [
    {
      "title": "Chapter 1 Title",
      "summary": "2-3 sentence summary of what happens",
      "key_events": ["event1", "event2"]
    }
  ]
}`;

    const userPrompt = `Create a ${chapter_count}-chapter outline for a ${genre || 'fiction'} novel.

Title: ${book.title}
Description: ${book.description || 'Not provided'}
${style ? `Writing Style: ${style}` : ''}
${metadata.braindump ? `Creative Ideas: ${metadata.braindump}` : ''}
${metadata.characters ? `Characters: ${metadata.characters}` : ''}
${metadata.synopsis ? `Story Synopsis: ${metadata.synopsis}` : ''}
${prompt ? `Additional Direction: ${prompt}` : ''}

Generate a compelling, well-paced outline with ${chapter_count} chapters. Each chapter should advance the plot meaningfully.`;

    // AI Provider configuration - supports Anthropic or Ollama
    const AI_PROVIDER = process.env.AI_PROVIDER || 'ollama';
    const OLLAMA_BASE_URL = process.env.OLLAMA_BASE_URL || 'http://192.168.1.200:11434';
    const AI_MODEL = process.env.AI_MODEL || 'deepseek-coder-v2:16b';
    const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;

    const isAnthropic = AI_PROVIDER === 'anthropic' && ANTHROPIC_API_KEY;
    const modelName = isAnthropic ? 'claude-sonnet-4-20250514' : AI_MODEL;

    if (!isAnthropic && !OLLAMA_BASE_URL) {
      console.error('No AI provider configured');
      return NextResponse.json({ error: 'AI service not configured' }, { status: 503 });
    }

    console.log(`Generating outline for book ${book_id} with ${chapter_count} chapters using ${AI_PROVIDER}/${modelName}...`);

    // Create generation log entry
    const logResult = await pool.query(
      `INSERT INTO generation_logs (book_id, generation_type, prompt, model, status)
       VALUES ($1, 'outline', $2, $3, 'processing')
       RETURNING id`,
      [book_id, userPrompt, modelName]
    );
    const logId = logResult.rows[0].id;

    // Call AI API (Anthropic or Ollama/OpenAI-compatible)
    let aiResponse: Response;

    if (isAnthropic) {
      // Anthropic Claude API
      aiResponse = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': ANTHROPIC_API_KEY!,
          'anthropic-version': '2023-06-01',
        },
        body: JSON.stringify({
          model: 'claude-sonnet-4-20250514',
          max_tokens: 8000,
          system: systemPrompt,
          messages: [{ role: 'user', content: userPrompt }],
        }),
      });
    } else {
      // Ollama OpenAI-compatible API
      aiResponse = await fetch(`${OLLAMA_BASE_URL}/v1/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: AI_MODEL,
          messages: [
            { role: 'system', content: systemPrompt },
            { role: 'user', content: userPrompt },
          ],
          max_tokens: 8000,
          temperature: 0.7,
          stream: false,
        }),
      });
    }

    if (!aiResponse.ok) {
      const errorText = await aiResponse.text();
      console.error('AI API error:', errorText);

      // Log the error
      await pool.query(
        `UPDATE generation_logs SET status = 'failed', error = $1, completed_at = NOW() WHERE id = $2`,
        [errorText, logId]
      );

      return NextResponse.json({ error: 'AI generation failed' }, { status: 500 });
    }

    const aiData = await aiResponse.json();
    
    // Extract text based on API format (Anthropic vs OpenAI/Ollama)
    let aiText: string;
    let inputTokens: number;
    let outputTokens: number;

    if (isAnthropic) {
      // Anthropic response format
      aiText = aiData.content[0]?.text || '';
      inputTokens = aiData.usage?.input_tokens || 0;
      outputTokens = aiData.usage?.output_tokens || 0;
    } else {
      // OpenAI/Ollama response format
      aiText = aiData.choices?.[0]?.message?.content || '';
      inputTokens = aiData.usage?.prompt_tokens || 0;
      outputTokens = aiData.usage?.completion_tokens || 0;
    }

    // Parse JSON from response (handle markdown code blocks)
    let outline: { synopsis: string; themes: string[]; chapters: GeneratedChapter[] };
    try {
      const jsonMatch = aiText.match(/```json\n?([\s\S]*?)\n?```/) || [null, aiText];
      outline = JSON.parse(jsonMatch[1] || aiText);
    } catch (parseError) {
      console.error('Failed to parse AI response:', aiText);

      // Log the parse error
      await pool.query(
        `UPDATE generation_logs SET status = 'failed', error = $1, result = $2,
         input_tokens = $3, output_tokens = $4, completed_at = NOW() WHERE id = $5`,
        [
          'Failed to parse AI response',
          JSON.stringify({ raw_response: aiText }),
          inputTokens,
          outputTokens,
          logId,
        ]
      );

      return NextResponse.json({ error: 'Failed to parse AI response' }, { status: 500 });
    }

    // Create chapters in database
    for (let i = 0; i < outline.chapters.length; i++) {
      const chapter = outline.chapters[i];
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
        .join('\n');

      const chaptersTable = hasContentSchema ? 'content.chapters' : 'chapters';
      await pool.query(
        `INSERT INTO ${chaptersTable} (book_id, chapter_number, title, content, word_count)
         VALUES ($1::uuid, $2, $3, $4, $5)`,
        [book_id, i + 1, chapter.title, content, content.split(/\s+/).length]
      );
    }

    const booksTable = hasContentSchema ? 'content.books' : 'books';
    const chaptersTable = hasContentSchema ? 'content.chapters' : 'chapters';
    await pool.query(
      `UPDATE ${booksTable} SET
        metadata = COALESCE(metadata, '{}'::jsonb) || $1,
        updated_at = NOW()
       WHERE id = $2::uuid`,
      [
        JSON.stringify({
          outline_generated: true,
          ai_synopsis: outline.synopsis,
          themes: outline.themes,
        }),
        book_id,
      ]
    );
    await pool.query(
      `UPDATE ${booksTable}
       SET word_count = (SELECT COALESCE(SUM(word_count), 0) FROM ${chaptersTable} WHERE book_id = $1::uuid)
       WHERE id = $1::uuid`,
      [book_id]
    );

    // Log successful generation
    await pool.query(
      `UPDATE generation_logs SET
         status = 'completed',
         result = $1,
         input_tokens = $2,
         output_tokens = $3,
         completed_at = NOW()
       WHERE id = $4`,
      [JSON.stringify(outline), inputTokens, outputTokens, logId]
    );

    console.log(
      `Outline generated: ${outline.chapters.length} chapters, ${inputTokens}/${outputTokens} tokens`
    );

    return NextResponse.json({
      success: true,
      synopsis: outline.synopsis,
      themes: outline.themes,
      chapters_created: outline.chapters.length,
      tokens: { input: inputTokens, output: outputTokens },
    });
  } catch (error) {
    console.error('Error generating outline:', error);
    return NextResponse.json({ error: 'Failed to generate outline' }, { status: 500 });
  } finally {
    await pool.end();
  }
}
