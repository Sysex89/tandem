# Deploy the hosted demo

The public demo runs on a small VPS (1 vCPU, 1 GB RAM) that already serves other sites through a Caddy container. Three pieces make that fit:

- **Images are built in CI, not on the server.** The *Images* workflow (`.github/workflows/images.yml`) builds the six service images on every push to `main` that touches `srcs/requirements/` and pushes them to GHCR as `ghcr.io/sysex89/tandem-<service>`, tagged `latest` and with the commit SHA. Compiling the Rust services or building Nuxt would not fit in 1 GB.
- **Only the app runs.** `deploy/docker-compose.yml` starts frontend, auth, notes, editor, nginx and Postgres (about 150 MB together). Grafana, Prometheus and the exporters stay local; the dashboards are documented in [the load test write-up](../explanation/load-test.md).
- **Caddy terminates TLS.** Tandem's nginx publishes no ports. It joins Caddy's Docker network as `tandem-nginx` and serves plain HTTP with `deploy/nginx.conf`, which uses the same route snippets as the local HTTPS server (`api-locations.conf`, `app-locations.conf`), so the `auth_request` gate is identical. Caddy obtains and renews the Let's Encrypt certificate.

```
Browser ──HTTPS/WSS──▶ Caddy ──HTTP──▶ nginx ──▶ frontend / auth / notes / editor ──▶ Postgres
                    (existing sites)    (tandem network, no published ports)
```

## One-time setup

1. **DNS:** an `A` record for the demo hostname pointing at the server.
2. **Swap**, as a safety net on a 1 GB machine:
   ```bash
   fallocate -l 2G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
   echo '/swapfile none swap sw 0 0' >> /etc/fstab
   ```
3. **Public images:** after the first *Images* run, set each `tandem-*` package to public (GitHub > Packages > package settings), so the server can pull without credentials.
4. **Local config:** `cp deploy/.env.deploy.example deploy/.env.deploy` and set `DEPLOY_HOST` (SSH target) and `DOMAIN`. The file is gitignored.

## Deploy

```bash
make deploy                        # latest images
TANDEM_TAG=<commit sha> make deploy  # a specific build, e.g. to roll back
```

`deploy/deploy.sh` copies the compose file and nginx config to `/opt/tandem`, generates `.env` and `secrets/` there on the first run (they never leave the server), pulls and starts the stack, appends a site block to the Caddyfile if the domain is not in it yet (validated before reloading; the previous file is kept as `Caddyfile.bak`), and seeds the `alice` / `bob` demo accounts. Re-running it is safe.

Wait for the *Images* workflow to finish after pushing before deploying, or `latest` is still the previous build.

## Operate

```bash
ssh <host> 'cd /opt/tandem && docker compose ps'
ssh <host> 'cd /opt/tandem && docker compose logs -f editor'
```

The database lives in the `tandem_pg18` volume on the server. GitHub login is disabled (`GITHUB_CLIENT_ID=demo-not-configured` in the server's `.env`); to enable it, register an OAuth app with the callback `https://<domain>/api/auth/login/github/callback`, put its ID in `.env` and its secret in `secrets/github_client_secret`, then `docker compose up -d auth`.
