import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'

const CONTENT_SERVICE_URL = process.env.CONTENT_SERVICE_URL
const SUBSCRIPTION_SERVICE_URL = process.env.SUBSCRIPTION_SERVICE_URL

interface BookSummary {
  word_count?: number | null
}

interface UsageResponse {
  usage?: { ai_words?: number; storage_gb?: number }
  limits?: { ai_words_per_month?: number; storage_gb?: number }
}

// GET /api/dashboard/stats - Aggregates books, words, and AI/storage usage for the dashboard.
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const authHeader = request.headers.get('authorization')!
  const baseUrl = request.nextUrl?.origin || ''

  const [booksResponse, usageResponse] = await Promise.allSettled([
    CONTENT_SERVICE_URL
      ? fetch(`${CONTENT_SERVICE_URL}/books`, {
          headers: { Authorization: authHeader, 'X-User-Id': userId },
          cache: 'no-store',
        })
      : fetch(`${baseUrl}/api/books?limit=500`, {
          headers: { Authorization: authHeader },
          cache: 'no-store',
        }),
    SUBSCRIPTION_SERVICE_URL
      ? fetch(`${SUBSCRIPTION_SERVICE_URL}/usage`, {
          headers: { Authorization: authHeader, 'X-User-Id': userId },
          cache: 'no-store',
        })
      : Promise.resolve(null),
  ])

  let totalBooks = 0
  let totalWords = 0
  if (booksResponse.status === 'fulfilled' && booksResponse.value?.ok) {
    const booksData = await booksResponse.value.json()
    const list: BookSummary[] = Array.isArray(booksData.books)
      ? booksData.books
      : Array.isArray(booksData)
        ? booksData
        : []
    totalBooks = typeof booksData.total === 'number' ? booksData.total : list.length
    totalWords = list.reduce((sum, book) => sum + (book.word_count || 0), 0)
  }

  let aiWordsUsed = 0
  let aiWordsLimit = 5000
  let storageUsedGb = 0
  let storageLimitGb = 1
  if (usageResponse.status === 'fulfilled' && usageResponse.value?.ok) {
    const usageData = (await usageResponse.value.json()) as UsageResponse
    aiWordsUsed = usageData.usage?.ai_words ?? 0
    aiWordsLimit = usageData.limits?.ai_words_per_month ?? aiWordsLimit
    storageUsedGb = usageData.usage?.storage_gb ?? 0
    storageLimitGb = usageData.limits?.storage_gb ?? storageLimitGb
  }

  // Fallback: derive AI usage from generation_jobs when no subscription service is wired up.
  if (!SUBSCRIPTION_SERVICE_URL) {
    try {
      const pool = getPool()
      const hasContentSchema = await pool
        .query(
          `SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'content' AND table_name = 'generation_jobs' LIMIT 1`
        )
        .then((r) => r.rows.length > 0)

      if (hasContentSchema) {
        const r = await pool.query(
          `SELECT COALESCE(SUM(g.credits_cost), 0) AS credits_used
             FROM content.generation_jobs g
             JOIN content.books b ON b.id = g.book_id
            WHERE b.author_id = $1::uuid`,
          [userId]
        )
        // Approximate words: ~10 words per credit for chapter generation jobs.
        aiWordsUsed = Number(r.rows[0]?.credits_used ?? 0) * 10
      }
    } catch (error) {
      console.warn('Dashboard stats DB fallback (usage) failed:', error)
    }
  }

  return NextResponse.json({
    totalBooks,
    totalWords,
    aiWordsUsed,
    aiWordsLimit,
    storageUsedGb,
    storageLimitGb,
    activeStreak: 0,
  })
}
