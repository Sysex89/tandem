# Monitoring architecture: Prometheus, Grafana, and application metrics

This page explains **why** the stack is shaped the way it is: pull scrapes, internal networking, file provisioning, and how application metrics fit next to nginx and postgres exporters.

For a hands-on first pass in the UI, use [Monitoring from zero (tutorial)](../tutorial/monitoring-from-zero.md).

## Pull model (Prometheus scrapes targets)

Prometheus **pulls** metrics from HTTP endpoints on a schedule. Each target exposes a text exposition format; Prometheus stores time series; Grafana queries Prometheus. That matches the pattern already used for **nginx-exporter** and **postgres-exporter** in this repo: one Prometheus process, many jobs, no custom push agent required for these services.

Application services (**auth**, **notes**, **editor**) follow the same contract: they expose `GET /metrics` on the Docker network so Prometheus can scrape them like any other target.

## Internal-only Prometheus

Prometheus is attached to the `transcendence` network and has **no published host port**. That is acceptable when Grafana is always the UI: operators use **Grafana Explore** or `docker compose exec` for debugging. Grafana itself has no port either; nginx proxies it at `/grafana/` on the same HTTPS origin as the app, so nginx is the only published service. Reducing exposed ports shrinks the attack surface.

## Grafana datasource and dashboards

The datasource is provisioned with a **fixed UID** (`prometheus`) so dashboard JSON can reference it reliably after fresh installs. Dashboards under `provisioning/dashboards/` are **file-backed** with provider settings that discourage ad-hoc edits in the UI (`disableDeletion`, `allowUiUpdates: false` in provider config). The intent is git-owned, reviewable dashboard definitions.

## Where `/metrics` lives on auth

The auth service registers `GET /metrics` **before** heavy plugins (database, cookies, OAuth) so scraping does not depend on migrations, secrets, or session state. Metrics remain available for health of the observability surface even when other subsystems misbehave.

## Label cardinality and security

Counters use **small fixed label sets** (`result`, `status`, `op`), not user IDs, emails, or IPs. That keeps Prometheus storage and query cost predictable and avoids turning the metrics layer into a PII leak.

Histograms for password verification time intentionally wrap **only** `argon2.verify` so the signal reflects hashing cost, not unrelated work.

## Nginx 502 and the auth `node_modules` volume

In `make dev`, compose bind-mounts the auth source tree but uses a **named volume** for `node_modules`. The running container therefore does not automatically pick up new packages from the host until `pnpm install` runs in that environment. If auth crashes on boot, nothing listens on port **3000**, and nginx reports **502 Bad Gateway** for `/api/auth/*`. The dev entrypoint plus `CI=true` exists so installs can run non-interactively in Docker.

## Relationship to infrastructure metrics

Postgres and nginx exporters describe **shared infrastructure**. Application metrics describe **this codebase’s behavior** (logins, registrations, note mutations, connected editors, saves). Together they answer different questions: “Is the DB up?” versus “Are users failing login?” Both are useful; neither replaces the other.

## Related docs

- [Add a new application metric (how-to)](../howto/add-a-new-application-metric.md)
- [Application and infrastructure metrics (reference)](../reference/application-and-infrastructure-metrics.md)
- [Verify Prometheus and Grafana metrics (how-to)](../howto/verify-prometheus-and-grafana-metrics.md)
- [Restore auth after new npm dependencies (how-to)](../howto/restore-auth-after-new-npm-dependencies.md)
