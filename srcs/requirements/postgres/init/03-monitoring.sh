#!/bin/bash
# Read-only role for postgres-exporter: pg_monitor grants access to the
# statistics views only, not to any table data.
set -e

if [ -f "/run/secrets/monitoring_db_password" ]; then
    MONITORING_DB_PASSWORD=$(cat /run/secrets/monitoring_db_password)
else
    echo "NO MONITORING SECRET FOUND"
    exit 1
fi

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE ROLE monitoring WITH LOGIN PASSWORD '$MONITORING_DB_PASSWORD';
    GRANT pg_monitor TO monitoring;
    GRANT CONNECT ON DATABASE $POSTGRES_DB TO monitoring;
EOSQL
