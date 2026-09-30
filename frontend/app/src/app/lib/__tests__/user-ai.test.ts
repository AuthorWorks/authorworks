import test from 'node:test'
import assert from 'node:assert/strict'

process.env.AI_KEY_ENCRYPTION_SECRET ||= 'test-secret-0123456789abcdef'

import { getUserAiOverride, validateInferenceUrl } from '../user-ai'
import { encryptSecret } from '../crypto'

test('validateInferenceUrl accepts public https endpoints', () => {
  for (const url of [
    'https://api.openai.com/v1',
    'https://my-gateway.example.com',
    'http://llm.example.com:8000/v1',
  ]) {
    const result = validateInferenceUrl(url)
    assert.equal(result.ok, true, url)
    if (result.ok) assert.equal(result.url, url)
  }
})

test('validateInferenceUrl rejects the audit-bypass forms', () => {
  // IPv4-mapped IPv6 loopback/private (previously passed hostname patterns)
  for (const url of [
    'http://[::ffff:127.0.0.1]/v1',
    'http://[::ffff:10.0.0.5]/v1',
    'http://[::ffff:169.254.169.254]/v1',
    // IPv6 forms previously unblocked
    'http://[fc00::1]/v1',
    'http://[fe80::1]/v1',
    'http://[::1]/v1',
    'http://[::]/v1',
    // classic IPv4
    'http://127.0.0.1:11434/v1',
    'http://10.0.0.1/v1',
    'http://172.16.0.9/v1',
    'http://192.168.1.10/v1',
    'http://169.254.169.254/latest/meta-data',
    // hostnames
    'http://localhost:11434/v1',
    'http://inference.cluster.local/v1',
    'http://db.internal:5432',
    // embedded credentials
    'https://user:pass@api.example.com/v1',
    // schemes
    'ftp://api.example.com/v1',
    'file:///etc/passwd',
    'gopher://127.0.0.1:6379/_INFO',
    // garbage
    'not a url',
  ]) {
    const result = validateInferenceUrl(url)
    assert.equal(result.ok, false, `${url} must be rejected`)
    if (!result.ok) assert.ok(result.error.length > 0)
  }
})

test('AI_ALLOW_PRIVATE_ENDPOINTS=true re-permits private endpoints (self-hosted)', () => {
  const prev = process.env.AI_ALLOW_PRIVATE_ENDPOINTS
  process.env.AI_ALLOW_PRIVATE_ENDPOINTS = 'true'
  try {
    assert.equal(validateInferenceUrl('http://10.0.0.5:11434/v1').ok, true)
    assert.equal(validateInferenceUrl('http://[::ffff:10.0.0.5]/v1').ok, true)
    // credentials and non-http(s) are STILL rejected in self-hosted mode
    assert.equal(validateInferenceUrl('https://user:pass@api.example.com/v1').ok, false)
    assert.equal(validateInferenceUrl('ftp://api.example.com/v1').ok, false)
  } finally {
    if (prev === undefined) delete process.env.AI_ALLOW_PRIVATE_ENDPOINTS
    else process.env.AI_ALLOW_PRIVATE_ENDPOINTS = prev
  }
})

test('getUserAiOverride is fail-closed until the generator path is guarded', async () => {
  // Stub pool: behaves like a real Pool for the settings lookup.
  const storedKey = encryptSecret('sk-user-key')
  const stubPool = {
    async query(sql: string, _params?: unknown[]) {
      if (/user_ai_settings/i.test(sql) && /SELECT/i.test(sql)) {
        return {
          rows: [
            {
              user_id: 'user-1',
              provider: 'openai-compatible',
              base_url: 'https://api.example.com/v1',
              model: 'gpt-4o-mini',
              api_key_encrypted: storedKey,
              enabled: true,
            },
          ],
        }
      }
      return { rows: [] } // CREATE TABLE / INSERT / UPDATE paths
    },
  } as never

  // Default: override suppressed even though a fully-configured row exists,
  // because the Rust book-generator consumer dials the endpoint without a
  // connect-time guard.
  delete process.env.BYOK_GENERATOR_GUARDED
  let warned = ''
  const origWarn = console.warn
  console.warn = (msg: string) => { warned = msg }
  try {
    assert.equal(await getUserAiOverride(stubPool, 'user-1'), null)
    assert.ok(warned.includes('fail-closed') || warned.includes('BYOK'), 'should warn operators')
  } finally {
    console.warn = origWarn
  }

  // Guard shipped and operator opted in: BYOK flows again, key decrypted.
  process.env.BYOK_GENERATOR_GUARDED = 'true'
  try {
    const override = await getUserAiOverride(stubPool, 'user-1')
    assert.ok(override, 'override expected when guarded')
    assert.equal(override!.baseUrl, 'https://api.example.com/v1')
    assert.equal(override!.apiKey, 'sk-user-key')
    assert.equal(override!.model, 'gpt-4o-mini')
  } finally {
    delete process.env.BYOK_GENERATOR_GUARDED
  }
})
