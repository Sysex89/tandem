# Restore auth after new npm dependencies

Goal: fix **502 Bad Gateway** from nginx on `/api/auth/*` when, under `make dev`, the auth container exits on startup with `Cannot find module '<package>'`. The production images (`make up`, `make demo`) install dependencies at build time and are not affected.

## Why this happens

The dev compose file (`srcs/docker-compose-dev.yml`) mounts the auth project at `/app` but uses a **named volume** for `/app/node_modules`. That volume keeps an older install. Adding a dependency on the host updates `package.json` and `pnpm-lock.yaml`, but the container still runs with the stale volume until dependencies are installed **inside** the container.

## What we ship to prevent repeat failures

The auth dev image's entrypoint (`docker-entrypoint-dev.sh`) runs `pnpm install` before `pnpm run dev`, with `CI=true` so pnpm does not require a TTY when it reconciles `node_modules`. Usually a restart is therefore enough.

If you changed the image (`Dockerfile.dev`) or entrypoint, rebuild the auth image.

## Steps

Rebuild and recreate the auth service:

```bash
docker compose -f srcs/docker-compose-dev.yml up -d --build auth
```

Watch logs until you see the server listening on port 3000:

```bash
docker compose -f srcs/docker-compose-dev.yml logs -f auth
```

Optional one-time reset of the named volume (destructive for cached modules only):

```bash
make down   # stops the stack, then removes only the node_modules and Nuxt cache volumes (not the database)
make dev
```

## Verify

From the nginx container:

```bash
docker compose -f srcs/docker-compose-dev.yml exec nginx curl -sS http://auth:3000/ping
```

Expect `pong`.

## Related docs

- [README_DEV.md](../../README_DEV.md): stale volumes and `make cleanv` / `make fclean`
- [Verify Prometheus and Grafana metrics](verify-prometheus-and-grafana-metrics.md)
