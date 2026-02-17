# Production Verification

This project includes two automated production verification workflows:

1. `E2E User Journey` (`.github/workflows/e2e-user-journey.yml`)
2. `Synthetic Production Monitor` (`.github/workflows/synthetic-prod-monitor.yml`)

## 1) E2E User Journey (CI)

Purpose: Validate real user flows against the live application.

Implemented flow:

1. Sign in through Logto UI
2. Create a book
3. Create and edit a chapter
4. Generate an outline
5. Verify chapter data is present after generation
6. Clean up synthetic test data

Schedule and triggers:

- Pushes to `main` affecting `frontend/app/**`
- Nightly scheduled run (`30 6 * * *`)
- Manual `workflow_dispatch`

Required secrets:

- `E2E_LOGTO_EMAIL`
- `E2E_LOGTO_PASSWORD`

Optional variables:

- `E2E_BASE_URL` (default: `https://author.works`)

Notes:

- Nightly runs enable full-book polling mode (`E2E_RUN_FULL_BOOK=true`).
- Push runs use the faster path (`E2E_RUN_FULL_BOOK=false`).

## 2) Synthetic Production Monitor

Purpose: Continuous synthetic checks for production regressions.

Implemented checks:

1. Homepage health
2. Frontend metrics endpoint health
3. Authenticated `auth/me` and dashboard stats checks
4. Synthetic CRUD flow for books and chapters
5. Cleanup of synthetic test data

Schedule and triggers:

- Every 15 minutes (`*/15 * * * *`)
- Manual `workflow_dispatch`

Required secrets:

- `SYNTHETIC_BEARER_TOKEN` (valid Logto access token for a monitor account)

Optional variables:

- `SYNTHETIC_BASE_URL` (default: `https://author.works`)

Failure handling:

- On failure, the workflow creates or updates a GitHub issue titled:
  - `Synthetic monitor failing for production`

## Operational recommendations

1. Create a dedicated low-privilege monitoring user for both workflows.
2. Rotate secrets and tokens regularly.
3. Route issue notifications to your on-call channel.
4. Keep cleanup enabled to avoid synthetic data buildup.
