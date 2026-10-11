#!/usr/bin/env bash
# Deploys the hosted demo: copies the production compose file and nginx config to
# the server, generates its .env and secrets on first run (they never leave the
# server), pulls the images built by the "Images" workflow, starts the stack,
# adds the site to the server's Caddy if it is missing, installs the nightly
# reset (reset.sh) and seeds alice and bob.
#
#   make deploy                      # latest images
#   TANDEM_TAG=<commit sha> make deploy
#
# Needs deploy/.env.deploy (gitignored), see deploy/.env.deploy.example.
set -euo pipefail

cd "$(dirname "$0")"
[ -f .env.deploy ] || { echo "Missing deploy/.env.deploy (copy .env.deploy.example)" >&2; exit 1; }
# shellcheck source=/dev/null
. ./.env.deploy
: "${DEPLOY_HOST:?}" "${DOMAIN:?}"
DIR=/opt/tandem
CADDY_DIR=${CADDY_DIR:-/opt/caddy}
TAG=${TANDEM_TAG:-latest}

echo "Copying the stack to $DIR..."
ssh "$DEPLOY_HOST" "install -d -m 700 $DIR $DIR/secrets"
rsync -az docker-compose.yml nginx.conf reset.sh ../scripts/demo-seed.sh "$DEPLOY_HOST:$DIR/"

echo "Starting tandem ($TAG)..."
ssh "$DEPLOY_HOST" DOMAIN="$DOMAIN" TAG="$TAG" DIR="$DIR" CADDY_DIR="$CADDY_DIR" bash -s <<-'REMOTE'
	set -euo pipefail
	cd "$DIR"
	rand() { head -c 32 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 32; }

	# First run only: generated here so the values never leave the server.
	if [ ! -f .env ]; then
		cat > .env <<-EOF
		DB_USER=tandem
		DB_PASSWORD=$(rand)
		DB_NAME=tandem
		# GitHub login is off; set a real OAuth app (and secrets/github_client_secret) to enable it.
		GITHUB_CLIENT_ID=demo-not-configured
		WEBSITE_URL=https://$DOMAIN
		EOF
		chmod 600 .env
	fi
	for s in auth_db_password monitoring_db_password session_secret_key; do
		[ -s "secrets/$s" ] || rand > "secrets/$s"
	done
	[ -s secrets/github_client_secret ] || printf demo-not-configured > secrets/github_client_secret
	# readable inside the containers (non-root users); the 700 directory keeps them private on the host
	chmod 644 secrets/*
	# recorded so later commands (and the nightly reset) keep this version
	sed -i '/^TANDEM_TAG=/d' .env
	echo "TANDEM_TAG=$TAG" >> .env

	docker compose pull -q
	docker compose up -d --remove-orphans
	docker image prune -f >/dev/null

	# Route the domain to tandem's nginx through Caddy, which handles the certificate.
	if ! grep -q "^$DOMAIN " "$CADDY_DIR/Caddyfile"; then
		cp "$CADDY_DIR/Caddyfile" "$CADDY_DIR/Caddyfile.bak"
		printf '\n%s {\n\tencode gzip zstd\n\treverse_proxy tandem-nginx:80\n}\n' "$DOMAIN" >> "$CADDY_DIR/Caddyfile"
		if ! docker exec caddy caddy validate --config /etc/caddy/Caddyfile >/dev/null 2>&1; then
			cp "$CADDY_DIR/Caddyfile.bak" "$CADDY_DIR/Caddyfile"
			echo "Caddy rejected the new site block; Caddyfile restored." >&2
			exit 1
		fi
		docker exec caddy caddy reload --config /etc/caddy/Caddyfile
		echo "Added $DOMAIN to Caddy"
	fi

	# Nightly reset at 04:00 server time: visitors' accounts and notes are wiped.
	echo "0 4 * * * root $DIR/reset.sh >> /var/log/tandem-reset.log 2>&1" > /etc/cron.d/tandem-reset
	chmod 644 /etc/cron.d/tandem-reset
REMOTE

if [ -z "$(dig +short "$DOMAIN" 2>/dev/null)" ]; then
	echo "$DOMAIN does not resolve yet: add its DNS record, then run make deploy again to seed the demo users."
	exit 0
fi
echo "Seeding the demo users..."
BASE="https://$DOMAIN" ../scripts/demo-seed.sh
