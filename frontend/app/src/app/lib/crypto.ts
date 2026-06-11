import { createCipheriv, createDecipheriv, createHash, randomBytes } from 'crypto'

/**
 * AES-256-GCM encryption for user-supplied AI API keys at rest.
 * The key is derived from AI_KEY_ENCRYPTION_SECRET (any sufficiently long
 * random string, provisioned via the deployment secret).
 */

const VERSION_PREFIX = 'v1'

function getKey(): Buffer {
  const secret = process.env.AI_KEY_ENCRYPTION_SECRET
  if (!secret || secret.length < 16) {
    throw new Error('AI_KEY_ENCRYPTION_SECRET is not configured (min 16 chars)')
  }
  return createHash('sha256').update(secret).digest()
}

export function isEncryptionConfigured(): boolean {
  const secret = process.env.AI_KEY_ENCRYPTION_SECRET
  return Boolean(secret && secret.length >= 16)
}

export function encryptSecret(plaintext: string): string {
  const key = getKey()
  const iv = randomBytes(12)
  const cipher = createCipheriv('aes-256-gcm', key, iv)
  const ciphertext = Buffer.concat([cipher.update(plaintext, 'utf8'), cipher.final()])
  const tag = cipher.getAuthTag()
  return [
    VERSION_PREFIX,
    iv.toString('base64'),
    tag.toString('base64'),
    ciphertext.toString('base64'),
  ].join(':')
}

export function decryptSecret(payload: string): string {
  const [version, ivB64, tagB64, dataB64] = payload.split(':')
  if (version !== VERSION_PREFIX || !ivB64 || !tagB64 || !dataB64) {
    throw new Error('Malformed encrypted payload')
  }
  const key = getKey()
  const decipher = createDecipheriv('aes-256-gcm', key, Buffer.from(ivB64, 'base64'))
  decipher.setAuthTag(Buffer.from(tagB64, 'base64'))
  return Buffer.concat([
    decipher.update(Buffer.from(dataB64, 'base64')),
    decipher.final(),
  ]).toString('utf8')
}
