# Agent Actions, Logging & Analytics

**Status:** Implemented for content/media workers; dashboard and monitoring in place.  
**K3s Homelab:** Reachable via `ssh alef`; deployment uses `k8s/overlays/homelab` and base-minimal (frontend + book-generator).

---

## 1. Agent (Worker) Actions & Data Recording

### Content worker (`workers/content`)

- **Jobs table:** `content.generation_jobs`
  - **Written by:** Content service (Spin/API) on outline/chapter/enhance requests.
  - **Columns used:** `id`, `book_id`, `job_type`, `status`, `input`, `output`, `error`, `started_at`, `completed_at`, `updated_at`, `credits_cost`, `credits_charged`.
- **Flow:**
  1. API inserts row with `status = 'pending'`, `input` (JSON).
  2. Content worker polls, sets `status = 'processing'`, `started_at = NOW()`.
  3. On success: `status = 'completed'`, `output` (JSON), `completed_at`.
  4. On failure: `status = 'failed'`, `error` (text), `completed_at`.
- **Credits:** Content service calls `record_job_credit_cost()` and `update_book_credits_used()` when enqueueing (credits consumed upfront). Worker does not write credit fields.

### Media worker (`workers/media`)

- **Jobs table:** `media.jobs`
  - Same pattern: `pending` → `processing` → `completed`/`failed`; `output`/`error`, `started_at`/`completed_at`, optional `progress`.

### Logging (stdout / Loki)

- Content worker uses **tracing** (`tracing_subscriber`):
  - `info!("Processing job: {} (type: {})", job.id, job.job_type)`
  - `info!("Job {} completed successfully", job.id)` / `error!("Job {} failed: {}", job.id, e)`
- Logs go to stdout; in K3s/Docker, Promtail can ship to Loki (see `config/promtail.yml`). No separate “agent action log” table beyond `generation_jobs` / `media.jobs`.

---

## 2. Analytics & Visualizations

### User-facing dashboard (`/dashboard`)

- **Source:** Next.js API route `/api/dashboard/stats`.
- **Behavior:**
  - **Books/words:** If `CONTENT_SERVICE_URL` is set, calls that service’s `/books`; otherwise uses same-origin `/api/books` (which uses DB and supports `content.books` with `author_id` or legacy `books` with `user_id`).
  - **Usage (AI words, storage):** If `SUBSCRIPTION_SERVICE_URL` is set, calls that service’s `/usage`. Otherwise **fallback:** reads from DB (`content.generation_jobs` joined with `content.books` by `author_id`) and derives an approximate “AI words” from `credits_cost * 10`.
- **Displayed:** Total books, total words, AI words used/limit, storage used/limit, streak (placeholder), recent books.

### Ops monitoring (Prometheus + Grafana)

- **Config:** `config/prometheus.yml`, `config/grafana/`, `k8s/base/monitoring.yaml`.
- **Metrics:** Service `/metrics` (e.g. request rate, latency, errors). Grafana dashboards and alerting rules are defined in `k8s/base/monitoring.yaml` (e.g. HighErrorRate, HighMemoryUsage, SlowResponseTime, ServiceDown).
- **Logs:** Promtail → Loki; optional correlation IDs if added in app logs.

---

## 3. Homelab (K3s) Checklist

- **Access:** `ssh alef`; cluster context typically `k3s-homelab` or as in `scripts/deploy-homelab.sh`.
- **Deploy:** `./scripts/deploy.sh homelab` or apply `k8s/overlays/homelab` (includes base-minimal: frontend, book-generator). Optional: `scripts/verify-homelab.sh` (Docker Compose); for K3s, use `kubectl get pods -n authorworks` etc.
- **Database:** PostgreSQL (e.g. `postgres.databases.svc.cluster.local`); run migrations (including `scripts/migrations/001_add_credit_system.sql`) so `content.generation_jobs` has `credits_cost`/`credits_charged` and subscription tables exist.
- **Content worker:** Must run and share `DATABASE_URL` with the same DB as the API so it can poll `content.generation_jobs`. In base-minimal homelab overlay, book-generator is present; ensure the **content worker** (job processor) is also deployed if you use outline/chapter/enhance generation.
- **Frontend env (K3s):** `CONTENT_SERVICE_URL` and `SUBSCRIPTION_SERVICE_URL` are optional. If unset, dashboard uses `/api/books` and DB fallback for usage so analytics still work.

---

## 4. Summary Table

| Area              | What’s recorded / used                    | Where                      |
|-------------------|-------------------------------------------|----------------------------|
| Agent job state   | status, input, output, error, timestamps   | `content.generation_jobs`, `media.jobs` |
| Credits per job   | credits_cost, credits_charged             | `content.generation_jobs` + subscription service |
| Agent logs        | Job start/complete/fail (tracing)         | stdout → Loki (if configured) |
| User analytics    | Books, words, AI usage, storage            | Dashboard via `/api/dashboard/stats` |
| Ops metrics       | Request rate, latency, errors             | Prometheus + Grafana       |
