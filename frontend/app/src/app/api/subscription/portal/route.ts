import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'

/**
 * GET /api/subscription/portal
 *
 * Redirects to a billing portal if one is configured. Two strategies are supported:
 *
 *  1) `SUBSCRIPTION_SERVICE_URL` is set: ask it for a portal URL for the user.
 *  2) `STRIPE_PORTAL_URL` is set: redirect directly (typical for static portals).
 *
 * When nothing is configured the route returns 503 so the UI can render an "unavailable"
 * state instead of taking the user to a broken page.
 */
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const subscriptionServiceUrl = process.env.SUBSCRIPTION_SERVICE_URL
  if (subscriptionServiceUrl) {
    try {
      const upstream = await fetch(`${subscriptionServiceUrl}/portal`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: request.headers.get('authorization') ?? '',
          'X-User-Id': userId,
        },
        cache: 'no-store',
      })
      if (upstream.ok) {
        const data = await upstream.json()
        if (data?.url) return NextResponse.redirect(data.url, 302)
      }
    } catch (error) {
      console.warn('Subscription portal lookup failed:', error)
    }
  }

  if (process.env.STRIPE_PORTAL_URL) {
    return NextResponse.redirect(process.env.STRIPE_PORTAL_URL, 302)
  }

  return NextResponse.json(
    { error: 'Billing portal is not configured for this deployment' },
    { status: 503 }
  )
}
