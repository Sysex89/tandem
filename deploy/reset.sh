#!/usr/bin/env bash
# Nightly reset of the hosted demo, run on the server by cron (installed by
# deploy.sh as /etc/cron.d/tandem-reset): drops the database volume, so every
# account and note made by visitors goes, restarts the stack on the deployed
# image tag and re-seeds alice, bob and their shared note. About 30 s of downtime.
set -euo pipefail
cd "$(dirname "$0")"

echo "$(date -u +%FT%TZ) resetting the demo"
# compose reads TANDEM_TAG from .env, so the deployed version is kept
docker compose down -v --remove-orphans
docker compose up -d
BASE=$(grep -E '^WEBSITE_URL=' .env | cut -d= -f2-) ./demo-seed.sh
