import { NextRequest, NextResponse } from 'next/server'

export async function POST(request: NextRequest) {
  // In a more complex setup, you might want to:
  // - Revoke the token with Logto
  // - Clear any server-side sessions
  // - Log the logout event

  return NextResponse.json({ success: true })
}

