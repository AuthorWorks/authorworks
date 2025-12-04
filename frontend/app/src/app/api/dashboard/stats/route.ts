import { NextRequest, NextResponse } from 'next/server'

const CONTENT_SERVICE_URL = process.env.CONTENT_SERVICE_URL || 'http://localhost:8080'
const SUBSCRIPTION_SERVICE_URL = process.env.SUBSCRIPTION_SERVICE_URL || 'http://localhost:8083'

export async function GET(request: NextRequest) {
  try {
    const authHeader = request.headers.get('authorization')
    if (!authHeader) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 })
    }

    // Fetch stats from various services in parallel
    const [booksResponse, usageResponse] = await Promise.allSettled([
      fetch(`${CONTENT_SERVICE_URL}/books`, {
        headers: { Authorization: authHeader, 'X-User-Id': request.headers.get('x-user-id') || '' },
      }),
      fetch(`${SUBSCRIPTION_SERVICE_URL}/usage`, {
        headers: { Authorization: authHeader, 'X-User-Id': request.headers.get('x-user-id') || '' },
      }),
    ])

    let totalBooks = 0
    let totalWords = 0
    if (booksResponse.status === 'fulfilled' && booksResponse.value.ok) {
      const booksData = await booksResponse.value.json()
      totalBooks = booksData.total || booksData.books?.length || 0
      totalWords = booksData.books?.reduce((sum: number, book: any) => sum + (book.word_count || 0), 0) || 0
    }

    let aiWordsUsed = 0
    let aiWordsLimit = 5000
    let storageUsedGb = 0
    let storageLimitGb = 1
    if (usageResponse.status === 'fulfilled' && usageResponse.value.ok) {
      const usageData = await usageResponse.value.json()
      aiWordsUsed = usageData.usage?.ai_words || 0
      aiWordsLimit = usageData.limits?.ai_words_per_month || 5000
      storageUsedGb = usageData.usage?.storage_gb || 0
      storageLimitGb = usageData.limits?.storage_gb || 1
    }

    return NextResponse.json({
      totalBooks,
      totalWords,
      aiWordsUsed,
      aiWordsLimit,
      storageUsedGb,
      storageLimitGb,
      activeStreak: 0, // Would be calculated from activity data
    })
  } catch (error) {
    console.error('Dashboard stats error:', error)
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    )
  }
}

