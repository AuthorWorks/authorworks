import test from 'node:test'
import assert from 'node:assert/strict'
import http from 'node:http'
import type { AddressInfo } from 'node:net'

import { assertHopAllowed, isPrivateAddress, safeFetch, SsrfError } from '../ssrf-guard'

const HOST = '127.0.0.1'

function startServer(
  handler: (req: http.IncomingMessage, res: http.ServerResponse) => void
): Promise<{ server: http.Server; port: number; close: () => Promise<void> }> {
  return new Promise((resolve) => {
    const server = http.createServer(handler)
    server.listen(0, HOST, () => {
      const { port } = server.address() as AddressInfo
      resolve({
        server,
        port,
        close: () => new Promise((done) => server.close(() => done())),
      })
    })
  })
}

test('isPrivateAddress blocks private/reserved IPv4 literals', () => {
  for (const ip of [
    '0.0.0.0',
    '10.0.0.1',
    '10.255.255.255',
    '100.64.0.1', // CGNAT
    '127.0.0.1',
    '127.8.8.8',
    '169.254.169.254', // cloud metadata
    '172.16.0.1',
    '172.31.255.255',
    '192.0.0.9',
    '192.0.2.10', // TEST-NET-1
    '192.168.1.1',
    '198.18.0.5',
    '198.51.100.7',
    '203.0.113.9',
    '224.0.0.1',
    '240.0.0.1',
  ]) {
    assert.equal(isPrivateAddress(ip), true, ip)
  }
})

test('isPrivateAddress allows public IPv4', () => {
  for (const ip of ['1.1.1.1', '8.8.8.8', '104.18.6.1', '172.32.0.1', '100.128.0.1', '193.0.0.1']) {
    assert.equal(isPrivateAddress(ip), false, ip)
  }
})

test('isPrivateAddress blocks IPv6 private/reserved forms, including mapped v4', () => {
  for (const ip of [
    '::',
    '::1',
    '::ffff:127.0.0.1', // the mapped-v4 bypass from the audit
    '::ffff:10.0.0.1',
    '::ffff:169.254.169.254',
    '::10.0.0.1', // deprecated v4-compatible
    'fd12:3456:789a::1', // ULA fd00::/8
    'fc00::1', // ULA fc00::/7 (was NOT blocked pre-patch)
    'fe80::1', // link-local (was NOT blocked pre-patch)
    '64:ff9b::1.2.3.4', // NAT64
    'ff02::1', // multicast
    '2001:db8::1', // documentation
    '[fd00::1]',
    'fe80::1%eth0', // zone id
  ]) {
    assert.equal(isPrivateAddress(ip), true, ip)
  }
})

test('isPrivateAddress allows public IPv6', () => {
  for (const ip of ['2606:4700::1111', '2a00:1450:4001:81::200e', '2600::1']) {
    assert.equal(isPrivateAddress(ip), false, ip)
  }
})

test('isPrivateAddress fails closed on garbage', () => {
  assert.equal(isPrivateAddress('not-an-ip'), true)
  assert.equal(isPrivateAddress(''), true)
  assert.equal(isPrivateAddress('999.1.1.1'), true)
  assert.equal(isPrivateAddress('::gg::1'), true)
})

test('assertHopAllowed rejects non-http(s), credentials, private hosts', () => {
  assert.throws(() => assertHopAllowed('ftp://example.com'), SsrfError)
  assert.throws(() => assertHopAllowed('file:///etc/passwd'), SsrfError)
  assert.throws(() => assertHopAllowed('https://user:pass@api.example.com'), SsrfError)
  assert.throws(() => assertHopAllowed('http://localhost:8080/v1'), SsrfError)
  assert.throws(() => assertHopAllowed('http://api.internal/v1'), SsrfError)
  assert.throws(() => assertHopAllowed('http://10.1.2.3/v1'), SsrfError)
  assert.throws(() => assertHopAllowed('http://[::ffff:10.1.2.3]/v1'), SsrfError)
  assert.throws(() => assertHopAllowed('http://[fe80::1]/v1'), SsrfError)
  // hostnames are fine at hop-check; validated at connect time
  assert.doesNotThrow(() => assertHopAllowed('https://api.openai.com/v1'))
})

test('safeFetch blocks connect-time to a LIVE private server (IPv4)', async () => {
  // A real listening server on 127.0.0.1: if enforcement were hostname-only
  // this would succeed. It must be refused before the request completes.
  const srv = await startServer((_req, res) => {
    res.writeHead(200, { 'content-type': 'text/plain' })
    res.end('internal-data')
  })
  try {
    await assert.rejects(
      safeFetch(`http://${HOST}:${srv.port}/v1/chat/completions`, { method: 'POST', body: '{}' }),
      SsrfError
    )
    // server never received the request
    assert.ok(true)
  } finally {
    await srv.close()
  }
})

test('safeFetch blocks connect-time to a LIVE private server (IPv6 loopback)', async () => {
  const srv = await startServer((_req, res) => {
    res.writeHead(200)
    res.end('internal-data')
  })
  try {
    await assert.rejects(
      safeFetch(`http://[::1]:${srv.port}/v1/models`),
      SsrfError
    )
  } finally {
    await srv.close()
  }
})

test('safeFetch follows redirects (per-hop re-validated) and enforces max hops', async () => {
  // Use the allow-private escape hatch to run the redirect machinery against
  // loopback servers (self-hosted mode), proving hop handling + cap.
  const prev = process.env.AI_ALLOW_PRIVATE_ENDPOINTS
  process.env.AI_ALLOW_PRIVATE_ENDPOINTS = 'true'
  try {
    const final = await startServer((_req, res) => {
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end('{"ok":true}')
    })
    const hop1 = await startServer((_req, res) => {
      res.writeHead(302, { location: `http://${HOST}:${final.port}/final` })
      res.end()
    })
    try {
      const res = await safeFetch(`http://${HOST}:${hop1.port}/start`, { method: 'GET' })
      assert.equal(res.status, 200)
      assert.equal(await res.text(), '{"ok":true}')

      // Redirect cap: 4 hops > max 3 -> error
      const h4 = await startServer((req, res) => {
        const n = Number((req.url ?? '/0').slice(1)) || 0
        res.writeHead(302, { location: `/${n + 1}` })
        res.end()
      })
      await assert.rejects(safeFetch(`http://${HOST}:${h4.port}/0`), SsrfError)
      await h4.close()
    } finally {
      await hop1.close()
      await final.close()
    }
  } finally {
    if (prev === undefined) delete process.env.AI_ALLOW_PRIVATE_ENDPOINTS
    else process.env.AI_ALLOW_PRIVATE_ENDPOINTS = prev
  }
})

test('AI_ALLOW_PRIVATE_ENDPOINTS=true permits private endpoints (self-hosted mode)', async () => {
  const prev = process.env.AI_ALLOW_PRIVATE_ENDPOINTS
  process.env.AI_ALLOW_PRIVATE_ENDPOINTS = 'true'
  try {
    const srv = await startServer((_req, res) => {
      res.writeHead(200)
      res.end('{"choices":[]}')
    })
    try {
      const res = await safeFetch(`http://${HOST}:${srv.port}/v1/chat/completions`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: '{}',
      })
      assert.equal(res.status, 200)
      const data: unknown = await res.json()
      assert.deepEqual(data, { choices: [] })
    } finally {
      await srv.close()
    }
  } finally {
    if (prev === undefined) delete process.env.AI_ALLOW_PRIVATE_ENDPOINTS
    else process.env.AI_ALLOW_PRIVATE_ENDPOINTS = prev
  }
})
