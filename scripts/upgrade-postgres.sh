#!/usr/bin/env bash
# Upgrades the database from Postgres 16 (volume mycelium_db) to Postgres 18
# (volume tandem_pg18) by dump and restore.
#
# A major Postgres version cannot open the previous one's data files, so the data
# is copied, never converted in place: the old volume is only read and stays as a
# backup (roll back by pointing the compose file at it again). Row counts of every
# table in both databases are compared before declaring success.
#
#   make upgrade-postgres
set -euo pipefail

cd "$(dirname "$0")/../srcs"

OLD=mycelium_db
NEW=tandem_pg18
OLD_IMAGE=postgres:16-alpine
NEW_IMAGE=postgres:18-alpine
DUMP_DIR=backups
env_value() { grep -E "^$1=" .env | head -1 | cut -d= -f2-; }
DB_USER=$(env_value DB_USER)
DB_PASSWORD=$(env_value DB_PASSWORD)

if ! docker volume inspect "$OLD" >/dev/null 2>&1; then
	echo "No $OLD volume: nothing to upgrade."
	exit 0
fi
if docker volume inspect "$NEW" >/dev/null 2>&1; then
	echo "$NEW already exists, so the upgrade has run (or a fresh 18 database was created)."
	echo "To redo it: docker volume rm $NEW, then run this again."
	exit 1
fi
version=$(docker run --rm -v "$OLD":/data alpine cat /data/PG_VERSION)
if [ "$version" != 16 ]; then
	echo "$OLD holds Postgres $version data, expected 16." >&2
	exit 1
fi

# Waits until the server accepts TCP connections. During first-time initialisation the
# image runs a temporary server on the Unix socket only, so this skips that phase.
wait_ready() {
	for _ in $(seq 1 60); do
		docker exec "$1" pg_isready -h localhost -U "$DB_USER" >/dev/null 2>&1 && return 0
		sleep 1
	done
	echo "$1 did not become ready" >&2
	docker logs "$1" | tail -20 >&2
	exit 1
}

# Row count of every table in every database, sorted, for comparing old and new.
row_counts() {
	local dbs
	dbs=$(docker exec "$1" psql -U "$DB_USER" -d postgres -Atc \
		"SELECT datname FROM pg_database WHERE NOT datistemplate AND datname <> 'postgres'")
	for db in $dbs; do
		docker exec "$1" psql -U "$DB_USER" -d "$db" -Atc "
			SELECT format('SELECT %L || ''|'' || count(*) FROM %I.%I;', '$db.' || schemaname || '.' || relname, schemaname, relname)
			FROM pg_stat_user_tables" | docker exec -i "$1" psql -U "$DB_USER" -d "$db" -At
	done | sort
}

cleanup() { docker rm -f tandem-pg16-dump tandem-pg18-restore >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "Stopping the stack (volumes are kept)..."
docker compose -f docker-compose.yml stop >/dev/null 2>&1 || true
docker compose -f docker-compose-dev.yml stop >/dev/null 2>&1 || true

echo "Dumping Postgres 16 from $OLD..."
docker run -d --name tandem-pg16-dump -v "$OLD":/var/lib/postgresql/data "$OLD_IMAGE" >/dev/null
wait_ready tandem-pg16-dump
mkdir -p "$DUMP_DIR"
dump="$DUMP_DIR/postgres16-$(date +%Y%m%d-%H%M%S).sql"
docker exec tandem-pg16-dump pg_dumpall -U "$DB_USER" > "$dump"
before=$(row_counts tandem-pg16-dump)
docker rm -f tandem-pg16-dump >/dev/null
echo "  $(wc -c < "$dump") bytes -> srcs/$dump"

echo "Restoring into Postgres 18 ($NEW)..."
# Same bootstrap superuser as before, so object ownership carries over. No init
# scripts and no POSTGRES_DB: the dump recreates every database and role itself.
docker run -d --name tandem-pg18-restore -v "$NEW":/var/lib/postgresql \
	-e POSTGRES_USER="$DB_USER" -e POSTGRES_PASSWORD="$DB_PASSWORD" -e POSTGRES_DB=postgres \
	"$NEW_IMAGE" >/dev/null
wait_ready tandem-pg18-restore
errors=$(docker exec -i tandem-pg18-restore psql -U "$DB_USER" -d postgres -q -v ON_ERROR_STOP=0 < "$dump" 2>&1 >/dev/null |
	grep -v -E "role \"$DB_USER\" already exists" || true)
if [ -n "$errors" ]; then
	echo "Restore reported errors:" >&2
	echo "$errors" >&2
	exit 1
fi
after=$(row_counts tandem-pg18-restore)

if [ "$before" != "$after" ]; then
	echo "Row counts differ between Postgres 16 and 18:" >&2
	diff <(echo "$before") <(echo "$after") >&2 || true
	exit 1
fi
echo "  $(echo "$after" | wc -l) tables, row counts match:"
echo "$after" | sed 's/^/    /'

echo
echo "Done. Start the stack with make up (it now uses $NEW)."
echo "$OLD is untouched and can be removed once you are happy: docker volume rm $OLD"
echo "The dump is in srcs/$dump."
