/**
 * Connection-time SSRF protection for user-supplied inference endpoints.
 *
 * Threat model (audit finding): hostname-string validation at save time does
 * not stop a "bring-your-own-inference" endpoint from reaching internal
 * services. DNS names can resolve to private addresses (rebinding), redirects
 * can bounce a validated request to an internal host, and IPv6 forms
 * (IPv4-mapped, unique-local, link-local) evade naive pattern checks.
 *
 * Enforcement here happens at actual connection time:
 *  - Literal IP hosts are range-checked before connecting.
 *  - Hostnames are resolved via a custom `lookup` hook: EVERY address the
 *    resolver returns is checked, and the socket is only allowed to connect
 *    to an address that passed the check. The IP the socket dials is therefore
 *    always a validated one - there is no TOCTOU window between validation
 *    and connection (no re-resolution race).
 *  - Redirects are followed manually (default max 3): each hop is
 *    re-validated through the same connect-time guard, so a public host that
 *    redirects to 10.x/127.x/[fd00::/8] is blocked.
 *
 * IPv4 blocked ranges: 0.0.0.0/8, 10.0.0.0/8, 100.64.0.0/10 (CGNAT),
 * 127.0.0.0/8, 169.254.0.0/16 (link-local, cloud metadata), 172.16.0.0/12,
 * 192.0.0.0/24, 192.0.2.0/24 (TEST-NET), 192.168.0.0/16, 198.18.0.0/15
 * (benchmarking), 198.51.100.0/24, 203.0.113.0/24, 224.0.0.0/4 (multicast),
 * 240.0.0.0/4 (reserved).
 *
 * IPv6 blocked ranges: :: (unspecified), ::1 (loopback), ::ffff:0:0/96 and
 * ::0.0.0.0/96 (IPv4-mapped/compatible -> recursive v4 check),
 * 64:ff9b::/96 (NAT64), fc00::/7 (unique local), fe80::/10 (link-local),
 * ff00::/8 (multicast), 2001:db8::/32 (documentation).
 *
 * Escape hatch: AI_ALLOW_PRIVATE_ENDPOINTS=true permits private endpoints
 * (self-hosted single-tenant deployments where internal inference is the
 * point). Fail-closed otherwise, including on resolver errors.
 */

import dns from 'node:dns'
import http from 'node:http'
import https from 'node:https'
import { URL } from 'node:url'

export class SsrfError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'SsrfError'
  }
}

/** Parse an IPv4 string into a 32-bit unsigned int, or null. */
function ipv4ToInt(ip: string): number | null {
  const parts = ip.split('.')
  if (parts.length !== 4) return null
  let value = 0
  for (const part of parts) {
    if (!/^\d{1,3}$/.test(part)) return null
    const n = Number(part)
    if (n > 255) return null
    value = value * 256 + n
  }
  return value
}

/** Parse an IPv6 string (may include "::" compression) into 16 bytes, or null. */
function ipv6ToBytes(ip: string): number[] | null {
  let addr = ip
  // Strip zone id (fe80::1%eth0)
  const zoneIdx = addr.indexOf('%')
  if (zoneIdx !== -1) addr = addr.slice(0, zoneIdx)

  const sections = addr.split('::')
  if (sections.length > 2) return null

  const head: string[] = sections[0] ? sections[0].split(':').filter(Boolean) : []
  const tail: string[] = sections.length === 2 && sections[1] ? sections[1].split(':').filter(Boolean) : []

  // Embedded IPv4 tail ("::ffff:1.2.3.4") becomes two hex groups.
  if (tail.length > 0 && tail[tail.length - 1].includes('.')) {
    const v4 = ipv4ToInt(tail[tail.length - 1])
    if (v4 === null) return null
    tail.pop()
    const hi = (v4 >>> 16) & 0xffff
    const lo = v4 & 0xffff
    tail.push(hi.toString(16), lo.toString(16))
  }

  for (const group of [...head, ...tail]) {
    if (!/^[0-9a-fA-F]{1,4}$/.test(group)) return null
  }

  const missing = 8 - head.length - tail.length
  if (sections.length === 2) {
    if (missing < 0) return null
  } else if (missing !== 0) {
    return null
  }

  const bytes: number[] = []
  const pushGroup = (g: string) => {
    const v = parseInt(g, 16)
    bytes.push((v >> 8) & 0xff, v & 0xff)
  }
  head.forEach(pushGroup)
  for (let i = 0; i < (sections.length === 2 ? missing : 0); i++) bytes.push(0, 0)
  tail.forEach(pushGroup)
  return bytes.length === 16 ? bytes : null
}

function v4InRanges(ip: string): boolean {
  const n = ipv4ToInt(ip)
  if (n === null) return true // unparseable -> treat as private (fail closed)
  const first = (n >>> 24) & 0xff
  const second = (n >>> 16) & 0xff
  if (first === 0 || first === 10 || first === 127) return true // this-network, private, loopback
  if (first === 100 && second >= 64 && second <= 127) return true // 100.64/10 CGNAT
  if (first === 169 && second === 254) return true // link-local (cloud metadata)
  if (first === 172 && second >= 16 && second <= 31) return true // 172.16/12
  if (first === 192 && second === 0) return true // 192.0.0/24 + 192.0.2/24 TEST-NET
  if (first === 192 && second === 168) return true
  if (first === 198 && (second === 18 || second === 19)) return true // benchmarking
  if (first === 198 && second === 51) return true // TEST-NET-2
  if (first === 203 && second === 0) return true // TEST-NET-3
  if (first >= 224) return true // multicast + reserved
  return false
}

/**
 * True when the address is private, loopback, link-local, unique-local,
 * IPv4-mapped-internal, multicast, reserved, or unparseable (fail closed).
 * Accepts IPv4 and IPv6 literals (brackets stripped by caller).
 */
export function isPrivateAddress(rawIp: string): boolean {
  const ip = rawIp.replace(/^\[|\]$/g, '').trim()
  if (!ip) return true

  if (ip.includes(':')) {
    const bytes = ipv6ToBytes(ip)
    if (!bytes) return true
    // All zero (:: and IPv4-compatible ::a.b.c.d)
    if (bytes.every((b) => b === 0)) return true
    // Loopback ::1
    if (bytes[15] === 1 && bytes.slice(0, 15).every((b) => b === 0)) return true
    // IPv4-mapped ::ffff:a.b.c.d and IPv4-compatible -> check the v4 part
    if (bytes.slice(0, 10).every((b) => b === 0) && bytes[10] === 0xff && bytes[11] === 0xff) {
      return v4InRanges(bytes.slice(12).join('.'))
    }
    if (bytes.slice(0, 12).every((b) => b === 0)) {
      // ::0.0.0.0/96 IPv4-compatible: deprecated, fail closed
      return true
    }
    // NAT64 64:ff9b::/96
    if (bytes[0] === 0x00 && bytes[1] === 0x64 && bytes[2] === 0xff && bytes[3] === 0x9b) return true
    // Unique local fc00::/7
    if ((bytes[0] & 0xfe) === 0xfc) return true
    // Link local fe80::/10
    if (bytes[0] === 0xfe && (bytes[1] & 0xc0) === 0x80) return true
    // Multicast ff00::/8
    if (bytes[0] === 0xff) return true
    // Documentation 2001:db8::/32
    if (bytes[0] === 0x20 && bytes[1] === 0x01 && bytes[2] === 0x0d && bytes[3] === 0xb8) return true
    return false
  }

  return v4InRanges(ip)
}

function privateEndpointsAllowed(): boolean {
  return process.env.AI_ALLOW_PRIVATE_ENDPOINTS === 'true'
}

// Hostnames that are private by name (cannot be resolved publicly). Resolvable
// public hostnames are validated at connect time via the lookup hook instead.
const PRIVATE_HOSTNAME_PATTERNS = [
  /^localhost$/i,
  /\.cluster\.local$/i,
  /\.internal$/i,
  /\.local$/i,
]

/**
 * Validates one hop of an outbound request BEFORE connecting: scheme and
 * literal-IP hosts. Hostnames are validated at connect time (see safeFetch).
 * Throws SsrfError on violation.
 */
export function assertHopAllowed(rawUrl: string | URL): URL {
  let parsed: URL
  try {
    parsed = rawUrl instanceof URL ? rawUrl : new URL(rawUrl)
  } catch {
    throw new SsrfError('Invalid URL')
  }
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    throw new SsrfError('URL must be http(s)')
  }
  if (parsed.username || parsed.password) {
    throw new SsrfError('Embedded credentials in URL are not allowed')
  }
  // Literal IP hosts (IPv4 or any IPv6 form) get the full range check now;
  // hostnames cannot be judged from the string and are instead validated at
  // connect time by the lookup hook (see nodeRequest).
  const hostname = parsed.hostname
  const literalIp = /^\d{1,3}(\.\d{1,3}){3}$/.test(hostname) || hostname.includes(':')
  if (
    !privateEndpointsAllowed() &&
    ((literalIp && isPrivateAddress(hostname)) ||
      PRIVATE_HOSTNAME_PATTERNS.some((p) => p.test(hostname)))
  ) {
    throw new SsrfError('Private or cluster-internal addresses are not allowed')
  }
  return parsed
}

function nodeRequest(
  url: URL,
  method: string,
  headers: Record<string, string>,
  body: string | undefined,
  timeoutMs: number
): Promise<{ status: number; headers: Record<string, string | string[] | undefined>; body: Buffer }> {
  return new Promise((resolve, reject) => {
    const isTls = url.protocol === 'https:'
    const transport = isTls ? https : http

    // Connect-time guard: every address the resolver returns is validated and
    // the socket dials the validated address. No re-resolution happens, so a
    // rebinding attacker cannot swap in a private IP after our check.
    const guardedLookup = (
      hostname: string,
      _options: dns.LookupOneOptions,
      callback: (err: Error | null, address: string, family: number) => void
    ) => {
      dns.lookup(hostname, { all: true }, (err, addresses) => {
        if (err) {
          callback(err, '', 4)
          return
        }
        const list = addresses as Array<{ address: string; family: number }>
        if (!privateEndpointsAllowed()) {
          for (const a of list) {
            if (isPrivateAddress(a.address)) {
              callback(
                new SsrfError(`Resolved address ${a.address} for ${hostname} is private or reserved`),
                '',
                a.family
              )
              return
            }
          }
        }
        if (list.length === 0) {
          callback(new SsrfError(`No addresses resolved for ${hostname}`), '', 4)
          return
        }
        callback(null, list[0].address, list[0].family)
      })
    }

    const req = transport.request(
      url,
      {
        method,
        headers,
        lookup: guardedLookup as never, // node types: net.SocketConnectOpts['lookup']
        servername: url.hostname, // SNI uses the original hostname, not the resolved IP
      },
      (res) => {
        const chunks: Buffer[] = []
        res.on('data', (c: Buffer) => chunks.push(c))
        res.on('end', () => {
          resolve({
            status: res.statusCode ?? 0,
            headers: res.headers as Record<string, string | string[] | undefined>,
            body: Buffer.concat(chunks),
          })
        })
        res.on('error', reject)
      }
    )
    req.setTimeout(timeoutMs, () => {
      req.destroy(new SsrfError(`Request timed out after ${timeoutMs}ms`))
    })
    req.on('error', reject)
    if (body !== undefined) req.write(body)
    req.end()
  })
}

const REDIRECT_STATUSES = [301, 302, 303, 307, 308]

export interface SafeFetchOptions {
  /** Maximum redirects followed (each re-validated). Default 3. */
  maxRedirects?: number
  /** Per-hop timeout in ms. Default 120000. */
  timeoutMs?: number
}

/**
 * fetch() replacement for user-supplied endpoints. Connect-time address
 * validation (see module docs), manual redirect following with per-hop
 * re-validation. Buffering the body is acceptable: consumers are
 * non-streaming LLM chat calls.
 */
export async function safeFetch(
  rawUrl: string,
  init?: { method?: string; headers?: Record<string, string>; body?: string },
  options?: SafeFetchOptions
): Promise<Response> {
  const maxRedirects = options?.maxRedirects ?? 3
  const timeoutMs = options?.timeoutMs ?? 120_000

  let url = assertHopAllowed(rawUrl)
  let method = (init?.method ?? 'GET').toUpperCase()
  let body = init?.body
  let redirects = 0

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const res = await nodeRequest(url, method, init?.headers ?? {}, body, timeoutMs)

    const location = res.headers.location
    if (REDIRECT_STATUSES.includes(res.status) && typeof location === 'string' && location) {
      if (redirects >= maxRedirects) {
        throw new SsrfError(`Too many redirects (max ${maxRedirects})`)
      }
      redirects++
      const next = new URL(location, url) // resolve relative redirects
      if (next.protocol !== 'https:' && next.protocol !== 'http:') {
        throw new SsrfError(`Refused redirect to non-http(s) protocol: ${next.protocol}`)
      }
      url = assertHopAllowed(next)
      if (res.status === 303 || ((res.status === 301 || res.status === 302) && method === 'POST')) {
        // Per RFC 7231 semantics for 303, and common-agent behavior for
        // POST->301/302: drop the body and switch to GET.
        method = 'GET'
        body = undefined
      }
      continue
    }

    const headerInit: Array<[string, string]> = []
    for (const [key, value] of Object.entries(res.headers)) {
      if (typeof value === 'string') headerInit.push([key, value])
      else if (Array.isArray(value)) value.forEach((v) => headerInit.push([key, v]))
    }
    // Copy out of the Node Buffer so the type is a plain ArrayBuffer view
    // (BodyInit) rather than Buffer<ArrayBufferLike>.
    const bytes = new Uint8Array(res.body.byteLength)
    bytes.set(res.body)
    return new Response(bytes, { status: res.status, headers: headerInit })
  }
}
