import { NextResponse } from 'next/server'

/**
 * Prometheus exposition format endpoint for homelab monitoring.
 * Scraped by Prometheus (kubernetes-pods discovery with prometheus.io annotations).
 */
export async function GET() {
  const timestamp = Math.floor(Date.now() / 1000)
  const body = [
    '# HELP authorworks_frontend_up Application is running (1 = up).',
    '# TYPE authorworks_frontend_up gauge',
    `authorworks_frontend_up 1 ${timestamp}`,
  ].join('\n')

  return new NextResponse(body, {
    status: 200,
    headers: {
      'Content-Type': 'text/plain; charset=utf-8; version=0.0.4',
      'Cache-Control': 'no-store',
    },
  })
}
