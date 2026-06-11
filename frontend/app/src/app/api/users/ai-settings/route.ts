import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import { getPool } from '@/app/lib/db'
import { encryptSecret, isEncryptionConfigured } from '@/app/lib/crypto'
import {
  ensureAiSettingsTable,
  getUserAiSettings,
  validateInferenceUrl,
} from '@/app/lib/user-ai'

// GET /api/users/ai-settings - Current user's AI provider settings (key never returned).
export async function GET(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  try {
    const settings = await getUserAiSettings(getPool(), userId)
    return NextResponse.json(settings)
  } catch (error) {
    console.error('GET /api/users/ai-settings failed:', error)
    return NextResponse.json({ error: 'Failed to fetch AI settings' }, { status: 500 })
  }
}

// PUT /api/users/ai-settings - Configure bring-your-own inference.
// body: { provider: 'default' } resets to platform inference.
// body: { provider: 'openai-compatible', base_url, model, api_key? }
//   api_key omitted/empty keeps the previously stored key.
export async function PUT(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: { provider?: string; base_url?: string; model?: string; api_key?: string }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const pool = getPool()

  try {
    await ensureAiSettingsTable(pool)

    if (body.provider === 'default') {
      await pool.query(
        `INSERT INTO public.user_ai_settings (user_id, provider, base_url, model, api_key_encrypted)
           VALUES ($1, 'default', NULL, NULL, NULL)
         ON CONFLICT (user_id) DO UPDATE SET
           provider = 'default', base_url = NULL, model = NULL,
           api_key_encrypted = NULL, updated_at = NOW()`,
        [userId]
      )
      return NextResponse.json(await getUserAiSettings(pool, userId))
    }

    if (body.provider !== 'openai-compatible') {
      return NextResponse.json(
        { error: "provider must be 'default' or 'openai-compatible'" },
        { status: 400 }
      )
    }

    if (!body.base_url || !body.model) {
      return NextResponse.json(
        { error: 'base_url and model are required for openai-compatible provider' },
        { status: 400 }
      )
    }

    const validation = validateInferenceUrl(body.base_url)
    if (!validation.ok) {
      return NextResponse.json({ error: validation.error }, { status: 400 })
    }

    let encryptedKey: string | null | undefined
    if (body.api_key) {
      if (!isEncryptionConfigured()) {
        return NextResponse.json(
          { error: 'API key storage is not configured on this deployment (AI_KEY_ENCRYPTION_SECRET missing)' },
          { status: 503 }
        )
      }
      encryptedKey = encryptSecret(body.api_key)
    }

    await pool.query(
      `INSERT INTO public.user_ai_settings (user_id, provider, base_url, model, api_key_encrypted)
         VALUES ($1, 'openai-compatible', $2, $3, $4)
       ON CONFLICT (user_id) DO UPDATE SET
         provider = 'openai-compatible',
         base_url = EXCLUDED.base_url,
         model = EXCLUDED.model,
         api_key_encrypted = COALESCE(EXCLUDED.api_key_encrypted, public.user_ai_settings.api_key_encrypted),
         updated_at = NOW()`,
      [userId, validation.url, body.model, encryptedKey ?? null]
    )

    return NextResponse.json(await getUserAiSettings(pool, userId))
  } catch (error) {
    console.error('PUT /api/users/ai-settings failed:', error)
    return NextResponse.json({ error: 'Failed to update AI settings' }, { status: 500 })
  }
}

// DELETE /api/users/ai-settings - Reset to platform default inference.
export async function DELETE(request: NextRequest) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  try {
    await ensureAiSettingsTable(getPool())
    await getPool().query(`DELETE FROM public.user_ai_settings WHERE user_id = $1`, [userId])
    return NextResponse.json({ provider: 'default', baseUrl: null, model: null, hasApiKey: false, updatedAt: null })
  } catch (error) {
    console.error('DELETE /api/users/ai-settings failed:', error)
    return NextResponse.json({ error: 'Failed to reset AI settings' }, { status: 500 })
  }
}
