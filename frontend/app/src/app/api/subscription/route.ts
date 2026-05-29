import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'

const PLAN_DEFAULTS: Record<string, { ai_words_per_month: number; storage_gb: number }> = {
  free: { ai_words_per_month: 5000, storage_gb: 1 },
  pro: { ai_words_per_month: 100000, storage_gb: 25 },
  enterprise: { ai_words_per_month: 1000000, storage_gb: 250 },
}

// GET /api/subscription - Returns the user's plan and usage. Proxies to a subscription service when configured.
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const subscriptionServiceUrl = process.env.SUBSCRIPTION_SERVICE_URL
  if (subscriptionServiceUrl) {
    try {
      const upstream = await fetch(`${subscriptionServiceUrl}/usage`, {
        headers: {
          Authorization: request.headers.get('authorization') ?? '',
          'X-User-Id': userId,
        },
        cache: 'no-store',
      })
      if (upstream.ok) {
        return NextResponse.json(await upstream.json())
      }
    } catch (error) {
      console.warn('Subscription service unreachable, using DB fallback:', error)
    }
  }

  const planId = (process.env.DEFAULT_SUBSCRIPTION_PLAN || 'free').toLowerCase()
  const limits = PLAN_DEFAULTS[planId] ?? PLAN_DEFAULTS.free

  let aiWordsUsed = 0
  let storageUsedGb = 0

  try {
    const pool = getPool()
    const hasContentSchema = await pool
      .query(
        `SELECT 1 FROM information_schema.tables
          WHERE table_schema = 'content' AND table_name = 'generation_jobs' LIMIT 1`
      )
      .then((r) => r.rows.length > 0)

    if (hasContentSchema) {
      const credits = await pool.query(
        `SELECT COALESCE(SUM(g.credits_cost), 0) AS used
           FROM content.generation_jobs g
           JOIN content.books b ON b.id = g.book_id
          WHERE b.author_id = $1::uuid
            AND g.created_at >= date_trunc('month', NOW())`,
        [userId]
      )
      aiWordsUsed = Number(credits.rows[0]?.used ?? 0) * 10
    }
  } catch (error) {
    console.warn('Subscription DB fallback failed:', error)
  }

  return NextResponse.json({
    plan_id: planId,
    status: 'active',
    usage: { ai_words: aiWordsUsed, storage_gb: storageUsedGb },
    limits,
  })
}
