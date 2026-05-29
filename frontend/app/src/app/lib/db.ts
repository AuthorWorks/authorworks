import { Pool } from 'pg'

declare global {
  // eslint-disable-next-line no-var
  var __authorworks_pg_pool__: Pool | undefined
}

const POOL_CONFIG = {
  connectionString: process.env.DATABASE_URL,
  max: Number(process.env.DATABASE_POOL_MAX ?? 10),
  idleTimeoutMillis: 30_000,
  connectionTimeoutMillis: 10_000,
}

// Reusing a singleton across hot reloads avoids exhausting Postgres connections in dev,
// and prevents per-request socket churn in production.
export function getPool(): Pool {
  if (!global.__authorworks_pg_pool__) {
    global.__authorworks_pg_pool__ = new Pool(POOL_CONFIG)
    global.__authorworks_pg_pool__.on('error', (err) => {
      console.error('Postgres pool error:', err)
    })
  }
  return global.__authorworks_pg_pool__
}
