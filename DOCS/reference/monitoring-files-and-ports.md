# Monitoring files and ports

Reference for where configuration lives and how services are reached. No procedural steps.

## Repository paths

| Item | Path |
|------|------|
| Prometheus scrape config | `srcs/requirements/prometheus/prometheus.yml` |
| Grafana datasource (Prometheus) | `srcs/requirements/grafana/provisioning/datasources/prometheus.yml` |
| Grafana dashboards (file provisioning) | `srcs/requirements/grafana/provisioning/dashboards/*.json` |
| Dashboard provider | `srcs/requirements/grafana/provisioning/dashboards/provider.yml` |
| Nginx (reverse proxy, `auth_request`, Grafana proxy) | `srcs/requirements/nginx/conf/nginx.conf` |
| Grafana admin credentials | `srcs/secrets/grafana_admin_user`, `srcs/secrets/grafana_admin_password` |
| Compose stack | `srcs/docker-compose.yml` |

## Host access

nginx is the only service with a published port:

| Service | Access |
|---------|--------|
| App | `https://localhost:8443` (self-signed certificate) |
| Grafana | `https://localhost:8443/grafana/`, proxied by nginx. `GF_SERVER_ROOT_URL` must match this origin, or Grafana Live rejects its WebSocket |
| Prometheus | Not published; use Grafana Explore or `docker compose exec` |

Internal DNS names on the `transcendence` network include `prometheus`, `grafana`, `nginx`, `auth`, `notes`, `editor`, `postgres`, `nginx-exporter`, `postgres-exporter`.

The hosted demo runs without the monitoring stack (see [Deploy the hosted demo](../howto/deploy-the-demo.md)).

## Prometheus datasource UID

Grafana provisioning sets datasource **UID** `prometheus`. Dashboard JSON references that UID.

## Nginx stub status (metrics)

Nginx listens on **8088** inside the container for `stub_status` (used by nginx-exporter); it is not published on the host.

## Postgres exporter

`postgres-exporter` connects as the read-only `monitoring` role (`pg_monitor`), created by `srcs/requirements/postgres/init/03-monitoring.sh` on fresh volumes. For an existing volume, see `README_DEV.md`.
