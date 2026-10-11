#!/usr/bin/env bash
# Generates srcs/.env and srcs/secrets/* for a local demo.
# Existing files are never overwritten, so a real configuration is left alone.
set -euo pipefail

cd "$(dirname "$0")/../srcs"

rand() { head -c 32 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 32; }

if [ -f .env ]; then
	echo "srcs/.env exists, leaving it untouched"
else
	cat > .env <<-EOF
	DB_USER=tandem
	DB_PASSWORD=$(rand)
	DB_NAME=tandem
	AUTH_DB_USER=auth
	AUTH_DB_NAME=auth
	POSTGRES_ADDR=postgres
	NOTES_ADDR=http://notes:3003
	AUTH_PORT=3000
	# GitHub login is disabled in the demo; set a real OAuth app here to enable it.
	GITHUB_CLIENT_ID=demo-not-configured
	GITHUB_CALLBACK_URL=https://localhost:8443/api/auth/login/github/callback
	WEBSITE_URL=https://localhost:8443
	UID=$(id -u)
	GID=$(id -g)
	EOF
	echo "Created srcs/.env"
fi

mkdir -p secrets
write_secret() {
	if [ ! -s "secrets/$1" ]; then
		printf '%s' "$2" > "secrets/$1"
		echo "Created srcs/secrets/$1"
	fi
}
write_secret auth_db_password "$(rand)"
write_secret monitoring_db_password "$(rand)"
write_secret session_secret_key "$(rand)"
write_secret github_client_secret "demo-not-configured"
write_secret grafana_admin_user "admin"
write_secret grafana_admin_password "$(rand)"
