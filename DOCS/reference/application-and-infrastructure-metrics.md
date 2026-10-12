# Application and infrastructure metrics

Reference for metric names, labels, HTTP paths, and scrape jobs. The source of truth is the code listed under each service.

## Scrape jobs (`prometheus.yml`)

| `job` label | Target |
|-------------|--------|
| `prometheus` | `prometheus:9090` |
| `nginx_exporter` | `nginx-exporter:9113` |
| `postgres_exporter` | `postgres-exporter:9187` |
| `notes` | `notes:3003` |
| `auth` | `auth:3000` |
| `editor` | `editor:3004` |

Global scrape interval: **5s** (see `prometheus.yml`).

## Application HTTP endpoints

| Service | Path | Content-Type (typical) |
|---------|------|-------------------------|
| Notes (Axum) | `GET /metrics` | Prometheus text exposition |
| Editor (Axum) | `GET /metrics` | Prometheus text exposition |
| Auth (Fastify) | `GET /metrics` | From `prom-client` registry |

These endpoints are for **internal** scraping on the Docker network; nginx does not route `/metrics` to these services, so they are not public.

## Notes service (Rust)

| Metric | Type | Labels | When incremented |
|--------|------|--------|------------------|
| `notes_mutations_total` | Counter | `op`: `create`, `delete` | After successful transaction commit (create) or delete with `rows_affected > 0` |

Implementation: `srcs/requirements/notes/src/metrics.rs`, `handlers.rs`.

## Editor service (Rust)

| Metric | Type | Labels / notes |
|--------|------|----------------|
| `editor_rooms_active` | Gauge | Documents loaded in memory |
| `editor_clients_connected` | Gauge | `access`: `read_write`, `read_only` (follows mid-session role changes) |
| `editor_connections_total` | Counter | `result`: `accepted`, `unauthorized` (no `X-User-Id`), `forbidden` (no access) |
| `editor_messages_total` | Counter | `kind`: `sync_step1`, `sync_step2`, `update`, `awareness`, `other`, `invalid` |
| `editor_updates_dropped_total` | Counter | Document updates from read-only clients that were dropped |
| `editor_access_revoked_total` | Counter | Open connections closed by the 15 s access re-check |
| `editor_clients_evicted_total` | Counter | Connections closed because the client's send queue was full |
| `editor_saves_total` | Counter | `trigger`: `periodic`, `last_client`, `shutdown`; `result`: `ok`, `error` |
| `editor_save_duration_seconds` | Histogram | `trigger` as above |
| `editor_document_size_bytes` | Histogram | Encoded document size at save time |
| `editor_title_syncs_total` | Counter | `result`: `ok`, `rejected`, `error` (title pushes to the notes service) |

Implementation: `srcs/requirements/editor/src/metrics.rs`. The connection and client labels are registered at zero on startup, so they are listed before first use.

## Auth service (Node)

| Metric | Type | Labels / notes |
|--------|------|----------------|
| `auth_login_local_total` | Counter | `result`: `success`, `fail_user_not_found`, `fail_no_local_account`, `fail_wrong_password` |
| `auth_password_verify_seconds` | Histogram | No labels; observes `argon2.verify` only when a local password hash exists |
| `auth_register_total` | Counter | `result`: `success`, `conflict_login`, `conflict_email`, `error` |
| `auth_github_callback_total` | Counter | `result`: `success_new_user`, `success_returning`, `conflict_email`, `error_oauth`, `error_github_api`, `error_create` |
| `auth_me_total` | Counter | `status`: `200`, `401`, `404` |
| `auth_verify_total` | Counter | `result`: `ok`, `unauthorized` (every nginx `auth_request`) |
| `auth_logout_total` | Counter | `result`: `ok` |

Implementation: `srcs/requirements/auth/src/metrics.ts`, `routes/auth.ts`, `routes/sessions.ts`, `routes/user.ts`.

## Provisioned Grafana dashboards

| Title | UID (file) |
|-------|------------|
| CPKNMS Overview (home dashboard) | `cpknms-overview` (`overview.json`) |
| Notes metrics | `notes-metrics` |
| Auth metrics | `auth-metrics` |
| Editor (real-time collaboration) | `editor-metrics` |

Dashboard queries filter with `job="notes"`, `job="auth"` or `job="editor"` where relevant.

## Related

- [Add a new application metric (how-to)](../howto/add-a-new-application-metric.md)
- [Monitoring files and ports](monitoring-files-and-ports.md)
- [Verify Prometheus and Grafana metrics (how-to)](../howto/verify-prometheus-and-grafana-metrics.md)
