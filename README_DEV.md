# Developer Guide

## Common Pitfalls

### 1. Stale Docker Volumes (The "Ghost in the Machine")

When we change service names, update configuration, or modify how `node_modules` are handled, Docker often **reuses** old volumes instead of creating new ones. This leads to weird bugs like:
- "Module not found" errors even after you `npm install`.
- Old code running even after you changed files.
- Configuration changes not taking effect.

**Solution: The Deep Clean**

If things act weird, your first step should be to purge the volumes. We have added specific Make commands for this.

*   **`make cleanv`**:
    *   **Use when**: Nuxt is acting up, or you changed `nuxt.config.ts` and it's not picking up, or `node_modules` seem broken.
    *   **What it does**: Deletes ONLY the `.nuxt`, `.output`, and frontend/auth `node_modules` volumes. (`make down` also runs this.)
    *   **Safety**: ✅ SAFE. It does NOT touch your Database.

*   **`make fclean`** (Full Clean):
    *   **Use when**: You want to reset the entire project to a "Fresh Clone" state.
    *   **What it does**: Deletes **EVERYTHING**, including the **Postgres Database** and **Auth Users**.
    *   **Safety**: ⚠️ DESTRUCTIVE. You will lose all data.

### 2. Permissions on Linux/Mac

If you see `EACCES` or permission denied errors, it's usually because files created inside the container (by `root`) are not editable by you (uid `1000` or `501`).

*   **Fix**: Add `UID=<output of id -u>` and `GID=<output of id -g>` to `srcs/.env`, then rebuild with `make dev`. The dev images (`docker-compose-dev.yml`) use these to map permissions correctly; they default to 1000.

### 3. Upgrading an existing database: monitoring role

`postgres-exporter` connects as a read-only `monitoring` role (`pg_monitor` only, no table access). Fresh volumes get it from `postgres/init/03-monitoring.sh`; init scripts don't re-run on existing volumes, so create it once by hand:

```bash
./scripts/demo-setup.sh   # creates srcs/secrets/monitoring_db_password if missing
set -a; . srcs/.env; set +a
docker exec -i postgres psql -U "$DB_USER" -d "$DB_NAME" \
  -c "CREATE ROLE monitoring LOGIN PASSWORD '$(cat srcs/secrets/monitoring_db_password)';" \
  -c "GRANT pg_monitor TO monitoring;"
make up
```

### 4. Upgrading from Postgres 16 to 18

Installations created before the upgrade keep their data in the `mycelium_db` volume (Postgres 16), which Postgres 18 cannot open. `make up`, `make demo` and `make dev` refuse to start until it is migrated:

```bash
make upgrade-postgres
```

It stops the stack, dumps the old database with a temporary Postgres 16 container, restores the dump into the new `tandem_pg18` volume with Postgres 18, and compares the row count of every table before and after. The old volume is only read, so it stays as a backup (roll back by pointing `postgres_data` in the compose files at `mycelium_db` and the image at `postgres:16-alpine`). The dump is kept in `srcs/backups/` (gitignored; it contains password hashes). Once you are happy: `docker volume rm mycelium_db`.
