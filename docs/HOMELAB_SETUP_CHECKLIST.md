# Homelab Setup Checklist (K3s)

Use this to ensure the stack is ready: login, create/edit books, AI outline and full-book generation.  
Server: **`ssh alef`** (K3s cluster).

---

## 1. Database migrations (order matters)

Run against cluster PostgreSQL (e.g. `postgres.databases.svc.cluster.local` or port-forward):

1. **Base schema** – Creates `content` (and optionally `public`) schema and tables. Use your existing init or `scripts/schema.sql` if present.

2. **Credit system**
   ```bash
   psql $DATABASE_URL -f scripts/migrations/001_add_credit_system.sql
   ```

3. **Frontend app tables**
   ```bash
   psql $DATABASE_URL -f scripts/migrations/002_frontend_schema.sql
   ```
   Creates `public.generation_logs` and, if missing, `public.books` and `public.chapters`. With only `content` schema, ensure `content.books` and `content.chapters` exist; the app uses them automatically.

**Verify:** `content.books`, `content.chapters`, `content.generation_jobs` (and optionally `public.generation_logs`, `public.books`, `public.chapters`).

---

## 2. Apply to K3s

From the repo root (or from `alef` with repo cloned):

```bash
./scripts/apply-homelab.sh
```

Or manually:

```bash
kubectl create namespace authorworks --dry-run=client -o yaml | kubectl apply -f -
# Apply secrets first if needed (see k8s/overlays/homelab/secrets-manual.yaml.example)
kubectl apply -k k8s/overlays/homelab
kubectl rollout status deployment/authorworks-frontend deployment/authorworks-book-generator deployment/authorworks-content-worker -n authorworks --timeout=300s
```

**Deployments:** `authorworks-frontend`, `authorworks-book-generator`, `authorworks-server` (simple-deployment), `authorworks-content-worker`, plus SealedSecrets.

---

## 3. Content worker

- **Image:** `ghcr.io/authorworks/content-worker:homelab` – built by CI (`.github/workflows/docker-homelab.yml`) on push to main.
- **Same DB as API:** Uses `authorworks-secrets` → `database-url`.
- **Enabled by default** in `k8s/overlays/homelab/kustomization.yaml`. To disable, remove `content-worker.yaml` from resources.

---

## 4. Monitoring (Prometheus, Loki, Uptime Kuma, Grafana)

- **Full guide:** [HOMELAB_MONITORING.md](HOMELAB_MONITORING.md) – Prometheus scrape/alert options, Loki (Vector already collects authorworks logs), Uptime Kuma monitors, Grafana/Refana.
- **Prometheus:** Cluster uses plain Prometheus (no Operator). Optional alerts: use [homelab-prometheus-alerts.yaml](homelab-prometheus-alerts.yaml) with kube-state-metrics or pod scrape.
- **Uptime Kuma:** Add a monitor for `https://author.works` (and API if used).
- **Loki:** No change; logs from `authorworks` namespace are already shipped by Vector.

---

## 5. User flows – quick check

| Flow | Check |
|------|--------|
| **Login** | Logto configured; frontend callback URL; users can sign in. |
| **Create book** | `POST /api/books`; book appears on dashboard. |
| **Edit book/chapters** | `GET/PUT /api/books/:id`, `GET/PUT/DELETE /api/chapters/:id`, `/api/books/:id/chapters`. |
| **AI outline** | `POST /api/generate/outline`; chapters and metadata updated. |
| **AI full book** | `POST /api/generate/book`; poll `GET /api/generate/book/status/:jobId`; sync updates chapters and `generation_logs`. |

---

## 6. Verify deployment (on alef after push)

After pushing to `main`, GitHub Actions builds images and (if secrets set) restarts deployments. ArgoCD will sync new manifests (content-worker, monitoring). On the cluster:

```bash
ssh alef
kubectl get pods -n authorworks
kubectl get deployments -n authorworks
kubectl rollout status deployment/authorworks-frontend deployment/authorworks-book-generator -n authorworks --timeout=60s
```

Expected: `authorworks-frontend`, `authorworks-book-generator` Running; `authorworks-content-worker` Running once the content-worker image has been built and ArgoCD/kustomize has synced. If content-worker is `ImagePullBackOff`, wait for the `Build Homelab Docker Images` workflow to finish (build-content-worker job).

## 7. Useful commands (on alef)

```bash
kubectl get pods -n authorworks
kubectl logs -n authorworks deployment/authorworks-content-worker --tail=50
kubectl rollout restart deployment/authorworks-frontend deployment/authorworks-book-generator deployment/authorworks-content-worker -n authorworks
```
