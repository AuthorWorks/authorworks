# Homelab monitoring integration (AuthorWorks)

Connect AuthorWorks to your existing stack: **Prometheus**, **Loki**, **Grafana**, **Uptime Kuma**, and **Tempo** (and other tools like Refana if you use them).

---

## 1. Prometheus

Your cluster uses **plain Prometheus** (config in `monitoring` namespace, e.g. `prometheus-config`), not the Prometheus Operator. AuthorWorks does not ship a `PrometheusRule` CRD so `kubectl apply` never fails on missing CRDs.

### Scraping AuthorWorks

- **Pod discovery:** Prometheus already has a `kubernetes-pods` job that scrapes any pod with:
  - `prometheus.io/scrape: "true"`
  - `prometheus.io/port: "<port>"`
  - `prometheus.io/path: "/metrics"`
- AuthorWorks workloads (frontend, book-generator, content-worker) do **not** expose `/metrics` today. To get `up` and custom metrics later, add a `/metrics` endpoint to the app and then add these annotations to the deployment pod template:
  ```yaml
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "3000"   # or 8081 for book-generator
    prometheus.io/path: "/metrics"
  ```

### Optional: static scrape job

If you prefer a fixed job name for AuthorWorks, add a scrape config to your `prometheus-config` (in the repo that manages `monitoring`, e.g. homelab-services):

```yaml
- job_name: authorworks
  static_configs:
    - targets:
        - authorworks-frontend.authorworks.svc.cluster.local:3000
        - authorworks-book-generator.authorworks.svc.cluster.local:8081
  metrics_path: /metrics
  # Only use if the targets expose /metrics; otherwise scrapes will fail.
```

### Alerts

Use the optional rule file in this repo so Prometheus (or Grafana Alerting) can fire when AuthorWorks is down:

- **File:** [docs/homelab-prometheus-alerts.yaml](homelab-prometheus-alerts.yaml)
- **Option A (recommended):** If you run **kube-state-metrics**, the first group uses `kube_deployment_status_replicas_available` for `authorworks-frontend`, `authorworks-book-generator`, and `authorworks-content-worker`.
- **Option B:** If you scrape AuthorWorks pods (with `/metrics` and annotations), use the second group (uncomment and adjust labels).

Add to your Prometheus `rule_files` or import the expressions into Grafana Alerting.

---

## 2. Loki (logs)

**Vector** in `monitoring` is already configured with `kubernetes_logs` and sends all pod logs to Loki with labels:

- `namespace` (e.g. `authorworks`)
- `pod`, `container`

AuthorWorks logs are therefore **already in Loki**. In Grafana Explore (Loki), use:

- `{namespace="authorworks"}` for all AuthorWorks logs
- Narrow by app: `{namespace="authorworks", pod=~"authorworks-frontend-.*"}` etc.

No change needed in AuthorWorks or Vector.

---

## 3. Uptime Kuma

Add monitors so you get availability and incident alerts:

| Monitor        | URL / type        | Notes                    |
|----------------|-------------------|--------------------------|
| AuthorWorks UI | `https://author.works` | HTTP(s), expect 200   |
| API (if used)  | `https://api.author.works` or your ingress | Same |
| Book-generator (internal) | `http://authorworks-book-generator.authorworks.svc.cluster.local:8081/health` | Only if Kuma runs in-cluster |

Create these in the Uptime Kuma UI; set interval and notifications to match your other apps.

---

## 4. Grafana (and Refana)

- **Prometheus:** Use your existing Prometheus datasource; add dashboards or alerts using the metrics and rules above.
- **Loki:** Use the same Loki datasource; filter by `namespace="authorworks"` for logs.
- **Tempo:** If you add tracing (OpenTelemetry) to AuthorWorks later, point Grafana to your existing Tempo datasource.
- **Refana / other:** If you use Refana or similar for referrer/analytics, integrate AuthorWorks the same way as your other apps (e.g. same tags or service name).

---

## 5. Summary

| System         | Action |
|----------------|--------|
| **Prometheus** | Optional: add scrape for AuthorWorks when `/metrics` exists; add [homelab-prometheus-alerts.yaml](homelab-prometheus-alerts.yaml) (e.g. via kube-state-metrics). |
| **Loki**       | No change; Vector already ships AuthorWorks logs. |
| **Uptime Kuma**| Add monitors for `https://author.works` (and API/health if desired). |
| **Grafana**    | Use existing Prometheus + Loki (and Tempo if you add tracing). |
| **Refana**     | Wire AuthorWorks like your other homelab apps. |

See [HOMELAB_SETUP_CHECKLIST.md](HOMELAB_SETUP_CHECKLIST.md) for apply and verify steps.
