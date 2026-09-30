# Auth & BYOK hardening — design notes

Status: implemented in this branch. Remediates the auth audit findings: bearer
tokens in localStorage, refresh tokens discarded, hostname-string-only
validation of user-supplied inference endpoints.

## Session model (HttpOnly server sessions)

- `POST /api/auth/callback` (PKCE flow unchanged) exchanges code+verifier at
  Logto, resolves the user via `/oidc/me`, and stores **both** the access and
  refresh token server-side in a new `public.auth_sessions` table (Postgres).
  Tokens are encrypted at rest with AES-256-GCM (`lib/crypto.ts`, keyed by
  `AI_KEY_ENCRYPTION_SECRET`). The browser receives only
  `{ user }` plus one opaque cookie:
  `aw_session = <32-byte random session id>.<HMAC-SHA256(session id)>`.
  HMAC key derived from `AUTH_SESSION_SECRET` (preferred) or
  `AI_KEY_ENCRYPTION_SECRET` (fallback, min 16 chars — fail-closed otherwise).
  No access/refresh token ever reaches client JS; there is no bearer token in
  localStorage.
- `lib/session.ts`: `createSession` / `loadSession` / `updateSessionTokens` /
  `revokeSession`. Rows carry `expires_at` (30 days) and `revoked_at`;
  `loadSession` only returns live (non-revoked, non-expired) sessions and
  fails closed (returns null) when decryption fails (e.g. rotated secret).
- Token renewal: when the stored access token is rejected by Logto,
  `resolveUser` exchanges the stored refresh token at `/oidc/token`
  (`grant_type=refresh_token`) and rotates the encrypted pair in the session
  row — the "refresh token returned but discarded" finding is fixed.
- `resolveUser` (lib/auth.ts) accepts either credential:
  1. `Authorization: Bearer …` header — validated directly at Logto
     (non-browser API clients; unchanged).
  2. The signed session cookie — origin-checked (below), token validated at
     Logto, transparent refresh when expired.

## Cookie flags

`aw_session`: `Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=2592000`.
Logout (`POST /api/auth/logout`) revokes the session row server-side, uses the
stored refresh token for Logto revocation when the body doesn't supply one,
and clears the cookie with `Max-Age=0`. The Logto SSO end-session URL is still
the browser's responsibility.

## CSRF defense

Non-GET/HEAD/OPTIONS requests authenticated via the **cookie** must present an
`Origin` (or `Referer`) header whose host equals the request host
(`x-forwarded-host` respected, first entry). Missing/ malformed headers fail
closed (`assertSameOrigin` in lib/auth.ts). Bearer-header requests are exempt —
not an ambient-credential CSRF vector. SameSite=Strict is defense in depth.

## Migration / cleanup behavior

- The `auth_sessions` table is created lazily (`CREATE TABLE IF NOT EXISTS`) on
  first session write/read — no migration script required. Existing sessions
  (localStorage bearer tokens) keep working through the Bearer path during
  transition; users simply re-login to pick up the cookie session.
- Revoked/expired rows are retained (audit trail) — a later cleanup job may
  `DELETE WHERE revoked_at < NOW() - INTERVAL '30 days'`.
- AuthProvider keeps an in-memory user (cleared with the tab); the legacy
  `accessToken` localStorage entry is removed on load.

## BYOK connect-time enforcement

- `lib/ssrf-guard.ts` `safeFetch` replaces `fetch` for **every**
  user-override inference call (lib/ai.ts; OpenAI-compatible path — overrides
  are OpenAI-compatible only, platform Anthropic keeps plain fetch):
  - Literal IP hosts range-checked before connect (full IPv4 private/reserved
    list incl. 169.254 metadata, CGNAT, TEST-NETs; IPv6 incl. `::1`,
    `::ffff:0:0/96` mapped, `fc00::/7`, `fe80::/10`, NAT64, `2001:db8::/32`).
  - Hostnames resolve through a custom `lookup` hook: **every** address the
    resolver returns is range-checked, and the socket dials a validated
    address — no re-resolution, so no DNS-rebinding TOCTOU window.
  - Redirects (max 3) are followed manually; each hop re-validated, including
    protocol downgrade refusal; embedded URL credentials rejected.
  - Fail-closed on unparseable addresses and resolver errors.
- Save-time validation (`validateInferenceUrl` in lib/user-ai.ts) is a fast
  pre-check only (literal IP ranges + `localhost`/`.local`/`.internal`/
  `.cluster.local`); real enforcement is at connect time.

## Feature flags

- `AI_ALLOW_PRIVATE_ENDPOINTS=true` — permits private endpoints (self-hosted
  single-tenant deployments where internal inference is the point). Default
  false.
- `BYOK_GENERATOR_GUARDED=true` — the ONLY switch that re-enables BYOK
  overrides. Default false.

## Out of scope (explicit dependency)

`/api/generate/book` forwards a user override's `base_url` + key to the **Rust
book-generator**, which dials the endpoint itself via the shared
homelab-inference crate with **no** connect-time SSRF guard. Until the
generator enforces connect-time address validation, `getUserAiOverride`
returns null (fail-closed, logs a warning) and generation falls back to the
platform gateway. `BYOK_GENERATOR_GUARDED=true` must only be set after that
generator patch ships.
