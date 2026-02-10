# Homelab monitoring integration (AuthorWorks)

Connect AuthorWorks to your existing stack: **Prometheus**, **Grafana**, **Loki**, **Uptime Kuma**, **Tempo**, and **Umami** (and other tools like Refana if you use them).

---

## 1. Prometheus

AuthorWorks **frontend** and **book-generator** expose Prometheus metrics and have pod annotations so your existing **kubernetes-pods** scrape job will discover them automatically.

### What’s in place

- **Frontend:** `GET /api/metrics` (Prometheus text format). Pod annotations: `prometheus.io/scrape=true`, `prometheus.io/port=3000`, `prometheus.io/path=/api/metrics`.
- **Book-generator:** `GET /metrics` (Prometheus text format). Pod annotations: `prometheus.io/scrape=true`, `prometheus.io/port=8081`, `prometheus.io/path=/metrics`.
- **Content-worker:** No HTTP server; use **Option A** alerts (kube-state-metrics) if you want alerts for it.

No change is required in your Prometheus scrape config; discovery is via pod annotations.

### Alert rules

Use `config/homelab-prometheus-alerts.yaml`:

- **Option B (default):** Alerts for frontend and book-generator based on `up{...}` from pod scrape (works with current setup).
- **Option A:** If you run **kube-state-metrics**, alerts also cover content-worker.

Add to your Prometheus `rule_files` (in the repo that manages `monitoring`, e.g. homelab-services), or import the expressions in **Grafana → Alerting**.

---

## 2. Grafana

- **Prometheus:** Use your existing Prometheus datasource; AuthorWorks targets appear as `up{kubernetes_namespace="authorworks", app="authorworks-frontend"}` and `...book-generator`.
- **Dashboard:** Import `config/grafana/authorworks-homelab-dashboard.json`: **Dashboards → Import → Upload JSON**. Set the Prometheus datasource when prompted. The dashboard shows frontend/book-generator status and a link to Loki logs.
- **Loki:** Use your existing Loki datasource; filter by `{namespace="authorworks"}` for all AuthorWorks logs.
- **Tempo:** If you add tracing to AuthorWorks later, use your existing Tempo datasource.

---

## 3. Loki (logs)

**Vector** in `monitoring` is already configured with `kubernetes_logs` and sends all pod logs to Loki with labels `namespace`, `pod`, `container`. AuthorWorks logs are **already in Loki**. In Grafana Explore (Loki):

- `{namespace="authorworks"}` — all AuthorWorks logs
- `{namespace="authorworks", pod=~"authorworks-frontend-.*"}` — frontend only
- `{namespace="authorworks", pod=~"authorworks-book-generator-.*"}` — book-generator only
- `{namespace="authorworks", pod=~"authorworks-content-worker-.*"}` — content-worker only

No change needed in AuthorWorks or Vector.

---

## 4. Uptime Kuma

Add these monitors in the Uptime Kuma UI so you get availability and incident alerts:

| Monitor name       | URL / type              | Notes                          |
|--------------------|--------------------------|--------------------------------|
| AuthorWorks UI     | `https://author.works`   | HTTP(s), expect 200             |
| AuthorWorks API    | `https://author.works/api/books` or your API base | Optional; 401/200 both “up” if auth required |
| Book-generator (internal) | `http://authorworks-book-generator.authorworks.svc.cluster.local:8081/health` | Only if Uptime Kuma runs in-cluster and can reach the service |

Create each monitor, set interval and notifications to match your other apps.

---

## 5. Other homelab services

| Service    | Integration |
|-----------|-------------|
| **Tempo** | Use existing Tempo datasource in Grafana when you add OpenTelemetry to AuthorWorks. |
| **Umami** | Add `author.works` (and any API host) as a website in Umami if you use it for analytics. |
| **Refana** | Wire AuthorWorks the same way as your other apps (e.g. same tags or service name). |
| **Homelab dashboard** | If it aggregates links or status, add AuthorWorks (e.g. link to https://author.works and to the Grafana dashboard). |

---

## 6. Summary checklist

| System         | Action |
|----------------|--------|
| **Prometheus** | No config change; pod annotations enable scrape. Optionally add `config/homelab-prometheus-alerts.yaml` to `rule_files` or Grafana Alerting. |
| **Grafana**    | Import `config/grafana/authorworks-homelab-dashboard.json`; use existing Prometheus + Loki datasources. |
| **Loki**       | No change; Vector already ships AuthorWorks logs. |
| **Uptime Kuma**| Add monitors for `https://author.works` (and API/health if desired). |
| **Tempo / Umami / Refana** | Use existing setup; add AuthorWorks when relevant. |

See [HOMELAB_SETUP_CHECKLIST.md](HOMELAB_SETUP_CHECKLIST.md) for apply and verify steps.
