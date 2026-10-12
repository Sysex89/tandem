# Verify Prometheus scrapes and Grafana dashboards

Goal: confirm that Prometheus is scraping targets and that Grafana can chart application metrics after `make up` or `make demo`.

If you have never opened Grafana in this project before, do [Monitoring from zero (tutorial)](../tutorial/monitoring-from-zero.md) first.

## Prerequisites

The stack is running. Grafana is at `https://localhost:8443/grafana/` (see [reference: Monitoring files and ports](../reference/monitoring-files-and-ports.md)).

## Confirm Prometheus targets

Prometheus has **no host port**; it only listens on the `transcendence` network. Use one of these approaches.

**Option A — Grafana Explore**

Open Grafana, choose the **Prometheus** datasource (UID `prometheus`), run `up`. You should see series with `job` labels `prometheus`, `nginx_exporter`, `postgres_exporter`, `notes`, `auth` and `editor`.

**Option B — Temporary port publish**

Run `docker compose -f srcs/docker-compose.yml exec prometheus wget -qO- http://localhost:9090/api/v1/targets` and inspect the JSON for `"health":"up"`.

## Confirm application metrics exist

In Grafana **Explore**, run:

- `notes_mutations_total` — counter for note creates/deletes (may be zero until you use the API).
- `auth_login_local_total` — counter for local login outcomes (zero until someone hits `POST /login` through the stack).
- `editor_clients_connected` — gauge of open WebSocket connections by access (registered at zero, so it shows up before anyone connects).

If the metric name returns no data but `up{job="notes"}` is `1`, the target is reachable; lack of series can mean no traffic yet or the metric was never registered (see explanation doc).

## Open provisioned dashboards

Under **Dashboards**, look for **CPKNMS Overview** (`cpknms-overview`, the home dashboard), **Notes metrics** (`notes-metrics`), **Auth metrics** (`auth-metrics`) and **Editor (real-time collaboration)** (`editor-metrics`). They are file-provisioned from `srcs/requirements/grafana/provisioning/dashboards/`.

## If a target stays down

Check the service listens inside Docker (`make logs service=<service>`). For **auth** under `make dev`, a missing dependency inside the `auth_node_modules` volume can prevent the process from binding port 3000; see [Restore auth after new npm dependencies](restore-auth-after-new-npm-dependencies.md).

## Related docs

- [Add a new application metric](add-a-new-application-metric.md)
- [Monitoring files and ports (reference)](../reference/monitoring-files-and-ports.md)
- [Application and infrastructure metrics (reference)](../reference/application-and-infrastructure-metrics.md)
- [Monitoring architecture (explanation)](../explanation/monitoring-architecture-prometheus-grafana.md)
