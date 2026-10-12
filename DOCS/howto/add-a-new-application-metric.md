# Add a new application metric

Goal: expose a new time series from **auth** (Node), **notes** or **editor** (Rust), have Prometheus scrape it, and confirm it in Grafana Explore. For background on pull scrapes and labels, see [Monitoring architecture (explanation)](../explanation/monitoring-architecture-prometheus-grafana.md).

## Before you change code

Pick a **metric name** that follows Prometheus conventions: ASCII letters, numbers, underscores; suffix `_total` for counters, `_seconds` for durations (often as a histogram). Plan **labels** in advance: use a **small fixed set** of values (for example `outcome="success|failure"`). Do **not** use user IDs, emails, session IDs, or free-text error messages as label values.

See [Application and infrastructure metrics (reference)](../reference/application-and-infrastructure-metrics.md) for existing names so you avoid collisions.

## Auth service (Fastify, prom-client)

**Define the metric** in `srcs/requirements/auth/src/metrics.ts`. Use `Counter`, `Histogram`, or `Gauge` from `prom-client`, with `registers: [prometheusRegister]` so it appears on `GET /metrics`.

**Increment or observe** in the relevant route handler under `srcs/requirements/auth/src/routes/`. Place the call on the code path that matches the semantic meaning (for example after a successful commit, or on each failure branch with a distinct `result` label).

**Rebuild runtime deps in Docker** if you develop with the auth container: new code does not add npm packages by itself, but if you added a library, rebuild auth per [Restore auth after new npm dependencies](restore-auth-after-new-npm-dependencies.md).

The `/metrics` route is already registered early in `srcs/requirements/auth/src/app.ts`; you do not need a new HTTP route for a new metric on the same registry.

## Notes service (Axum, prometheus crate)

**Register the metric** in `srcs/requirements/notes/src/metrics.rs`. Use `CounterVec` / `HistogramVec` when you need labels, or a single `Counter` / `Histogram` when you do not. Register on `prometheus::default_registry()` once (the existing `OnceLock` pattern avoids double registration).

**Call `init()`** from `main.rs` if you add a new collector that must exist before first scrape even at zero; otherwise ensure something registers it on startup.

**Increment or observe** from `handlers.rs` (or a small helper) at the point where the event actually happened (for example after `commit()` succeeds).

**Expose** metrics through the existing `gather_prometheus_text()` path and `GET /metrics` in `srcs/requirements/notes/src/main.rs` unless you intentionally use a separate registry (not the current pattern).

## Editor service (Axum, prometheus crate)

All editor metrics live in one `Metrics` struct in `srcs/requirements/editor/src/metrics.rs`, built once behind `get()`. Add a field, register it in the constructor with an `editor_` prefix, and use it with `metrics::get().<field>`. If a labelled metric should be listed before its first event, touch its label values in `init()` (called from `main.rs`), as is done for `editor_clients_connected`. `GET /metrics` is already served from `main.rs`.

If the metric belongs on the editor dashboard, add a panel to `grafana/provisioning/dashboards/editor-metrics.json`.

## New service (not auth, notes or editor)

Implement **`GET /metrics`** in that service (Prometheus text exposition). Add a **`scrape_configs`** entry in `srcs/requirements/prometheus/prometheus.yml` with a distinct `job_name` and the container **hostname** and **port** on the `transcendence` network. `prometheus.yml` is bind-mounted, so `make rebuild service=prometheus` (or a restart) picks it up.

## Wire Prometheus and verify

After the service listens and exposes the metric, confirm the target is **UP** (see [Verify Prometheus and Grafana metrics](verify-prometheus-and-grafana-metrics.md)). In Grafana Explore, query the new metric name. If the series is missing but `up{job="..."} == 1`, trigger the code path once counters may start at zero only after first use—histograms and counters usually appear after the first event.

## Optional: dashboard panel

Add a panel to a provisioned JSON under `srcs/requirements/grafana/provisioning/dashboards/`. A panel built in the UI can be exported as JSON and pasted in. The provisioning folder is copied into the Grafana image, so run `make rebuild service=grafana` afterwards. Use the same `job="auth"`, `job="notes"` or `job="editor"` selector as the other app panels. Dashboards are provisioned with `allowUiUpdates: false`, so edit the JSON in the repository rather than in the UI.

## Related docs

- [Application and infrastructure metrics (reference)](../reference/application-and-infrastructure-metrics.md)
- [Monitoring files and ports (reference)](../reference/monitoring-files-and-ports.md)
- [Verify Prometheus and Grafana metrics](verify-prometheus-and-grafana-metrics.md)
