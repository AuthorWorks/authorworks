import { NextRequest, NextResponse } from 'next/server'

// GET /api/generate/book/status/:jobId - Get job status from book generator
export async function GET(
  request: NextRequest,
  { params }: { params: { jobId: string } }
) {
  const BOOK_GENERATOR_URL = process.env.BOOK_GENERATOR_URL || 'http://authorworks-book-generator.authorworks.svc.cluster.local:8081'
  const { jobId } = params

  console.log('GET /api/generate/book/status - jobId:', jobId)

  try {
    const response = await fetch(`${BOOK_GENERATOR_URL}/api/jobs/${jobId}`, {
      headers: {
        'Content-Type': 'application/json',
      },
    })

    if (!response.ok) {
      if (response.status === 404) {
        return NextResponse.json({ error: 'Job not found' }, { status: 404 })
      }
      return NextResponse.json({ error: 'Failed to get job status' }, { status: 500 })
    }

    const status = await response.json()
    return NextResponse.json(status)
  } catch (error) {
    console.error('GET /api/generate/book/status - Error:', error)
    return NextResponse.json({ error: 'Internal server error' }, { status: 500 })
  }
}
