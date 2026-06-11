import { NextResponse } from 'next/server'
import { getPublicAuthConfig } from '@/app/lib/auth'

export const dynamic = 'force-dynamic'

// GET /api/auth/config - Runtime public OIDC config for the browser.
// Avoids baking NEXT_PUBLIC_* values into the image at build time.
export async function GET() {
  return NextResponse.json(getPublicAuthConfig())
}
