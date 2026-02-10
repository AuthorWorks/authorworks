import { NextRequest, NextResponse } from 'next/server'
import { Pool } from 'pg'

const CONTENT_SERVICE_URL = process.env.CONTENT_SERVICE_URL
const SUBSCRIPTION_SERVICE_URL = process.env.SUBSCRIPTION_SERVICE_URL

function getPool() {
  return new Pool({
    connectionString:
      process.env.DATABASE_URL ||
      'postgresql://postgres:homelab_postgres_2024@postgres.databases.svc.cluster.local:5432/authorworks',
  })
}

async function getUserId(request: NextRequest): Promise<string | null> {
  const authHeader = request.headers.get('authorization')
  if (!authHeader?.startsWith('Bearer ')) return null
  const token = authHeader.substring(7)
  const LOGTO_ENDPOINT =
    process.env.LOGTO_ENDPOINT || 'http://logto.security.svc.cluster.local:3001'
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

export async function GET(request: NextRequest) {
  try {
    const authHeader = request.headers.get('authorization')
    if (!authHeader) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
    }

    const userId = await getUserId(request)
    if (!userId) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
    }

    // Prefer external services when configured; otherwise use same-origin API and DB fallback
    const baseUrl = request.nextUrl?.origin || ''
    const [booksResponse, usageResponse] = await Promise.allSettled([
      CONTENT_SERVICE_URL
        ? fetch(`${CONTENT_SERVICE_URL}/books`, {
            headers: { Authorization: authHeader, 'X-User-Id': userId },
          })
        : fetch(`${baseUrl}/api/books?limit=500`, {
            headers: { Authorization: authHeader },
          }),
      SUBSCRIPTION_SERVICE_URL
        ? fetch(`${SUBSCRIPTION_SERVICE_URL}/usage`, {
            headers: { Authorization: authHeader, 'X-User-Id': userId },
          })
        : Promise.resolve(null),
    ])

    let totalBooks = 0
    let totalWords = 0
    if (booksResponse.status === 'fulfilled' && booksResponse.value?.ok) {
      const booksData = await (booksResponse.value as Response).json()
      const books = booksData.books ?? booksData
      const list = Array.isArray(books) ? books : []
      totalBooks = booksData.total ?? list.length
      totalWords = list.reduce((sum: number, book: any) => sum + (book.word_count || 0), 0)
    }

    let aiWordsUsed = 0
    let aiWordsLimit = 5000
    let storageUsedGb = 0
    let storageLimitGb = 1
    if (usageResponse.status === 'fulfilled' && usageResponse.value) {
      const res = usageResponse.value as Response
      if (res.ok) {
        const usageData = await res.json()
        aiWordsUsed = usageData.usage?.ai_words ?? 0
        aiWordsLimit = usageData.limits?.ai_words_per_month ?? 5000
        storageUsedGb = usageData.usage?.storage_gb ?? 0
        storageLimitGb = usageData.limits?.storage_gb ?? 1
      }
    }

    // Fallback: derive usage from DB when subscription service is not used (e.g. homelab minimal)
    if (!SUBSCRIPTION_SERVICE_URL || (usageResponse.status === 'fulfilled' && !usageResponse.value)) {
      const pool = getPool()
      try {
        // Credits used from generation_jobs (content schema) or credits (subscriptions schema)
        const hasContentSchema = await pool
          .query(
            `SELECT 1 FROM information_schema.tables WHERE table_schema = 'content' AND table_name = 'generation_jobs' LIMIT 1`
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
          const creditsUsed = Number(r.rows[0]?.credits_used ?? 0)
          // Approximate AI words: ~10 words per credit for chapter gen
          aiWordsUsed = creditsUsed * 10
        }
      } catch (e) {
        console.warn('Dashboard stats DB fallback (usage):', e)
      } finally {
        await pool.end()
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
  } catch (error) {
    console.error('Dashboard stats error:', error)
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    )
  }
}

