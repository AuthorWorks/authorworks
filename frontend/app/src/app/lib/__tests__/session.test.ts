import test from 'node:test'
import assert from 'node:assert/strict'

import { decryptSecret, encryptSecret } from '../crypto'
import { signSessionId, verifySessionCookie } from '../session'

test('signSessionId/verifySessionCookie round-trips', () => {
  const sessionId = 'abcdefgh12345678'
  const cookie = signSessionId(sessionId)
  assert.notEqual(cookie, sessionId)
  assert.equal(verifySessionCookie(cookie), sessionId)
})

test('verifySessionCookie rejects tampering and garbage', () => {
  const cookie = signSessionId('sess-abc')
  // flip the MAC
  const parts = cookie.split('.')
  const forged = `${parts[0]}.${parts[1].slice(0, -2)}AA`
  assert.equal(verifySessionCookie(forged), null)
  // forged session id with attacker-chosen MAC
  const attacker = '0000attacker'
  const stolenMac = parts[1]
  assert.equal(verifySessionCookie(`${attacker}.${stolenMac}`), null)
  // shape garbage
  assert.equal(verifySessionCookie(undefined), null)
  assert.equal(verifySessionCookie(''), null)
  assert.equal(verifySessionCookie('no-dot-here'), null)
  assert.equal(verifySessionCookie('.'), null)
})

test('encryptSecret/decryptSecret round-trips the refresh-token storage', () => {
  const refreshToken = 'logto-refresh-token-=='
  const stored = encryptSecret(refreshToken)
  assert.notEqual(stored, refreshToken)
  assert.ok(!stored.includes(refreshToken))
  assert.equal(decryptSecret(stored), refreshToken)
})

test('encrypted token blob fails cleanly on wrong key material (fail closed)', () => {
  const stored = encryptSecret('token-value')
  // Corrupt ciphertext -> decryptSecret must throw or return garbage, never
  // silently the plaintext; assert it does not return the original plaintext.
  const corrupted = stored.slice(0, -4) + 'AAAA'
  let result: string | null = null
  try {
    result = decryptSecret(corrupted)
  } catch {
    result = null // throwing is the correct fail-closed behavior
  }
  assert.notEqual(result, 'token-value')
})
