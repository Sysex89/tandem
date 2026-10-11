#!/usr/bin/env bash
# Seeds two demo users and a note shared between them (editor access).
# Safe to re-run: existing users are logged in instead of re-registered,
# and the note is only created if alice has no notes yet.
set -euo pipefail

BASE="${BASE:-https://localhost:8443}"
PASSWORD="tandem-demo"
JAR_DIR="$(mktemp -d)"
trap 'rm -rf "$JAR_DIR"' EXIT

api() { curl -sk -b "$JAR_DIR/$1" -c "$JAR_DIR/$1" -H 'Content-Type: application/json' "${@:2}"; }
first_id() { grep -o '"id":"[0-9a-f-]*"' | head -1 | cut -d'"' -f4; }

echo "Waiting for $BASE ..."
for _ in $(seq 1 90); do
	[ "$(curl -sk -o /dev/null -w '%{http_code}' "$BASE/api/auth/me")" = "401" ] && break
	sleep 2
done

account() { # name
	local code
	code=$(api "$1" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/auth/register" \
		-d "{\"loginName\":\"$1\",\"email\":\"$1@demo.local\",\"password\":\"$PASSWORD\"}")
	if [ "$code" = "409" ]; then
		code=$(api "$1" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/auth/login" \
			-d "{\"identifier\":\"$1\",\"password\":\"$PASSWORD\"}")
	fi
	[ "$code" = "200" ] || { echo "Could not sign in as $1 (HTTP $code)" >&2; exit 1; }
}

account alice
account bob
BOB_ID=$(api bob "$BASE/api/auth/me" | first_id)

if [ -z "$(api alice "$BASE/api/notes" | first_id)" ]; then
	NOTE_ID=$(api alice -X POST "$BASE/api/notes" -d '{"title":"Project kickoff"}' | first_id)
	api alice -o /dev/null -X POST "$BASE/api/notes/collab/" \
		-d "{\"note_id\":\"$NOTE_ID\",\"guest_id\":\"$BOB_ID\",\"role\":\"Edit\"}"
fi

cat <<EOF

Demo ready at $BASE (accept the self-signed certificate)
  alice / $PASSWORD   owns "Project kickoff"
  bob   / $PASSWORD   can edit it (Notes > Shared with me)
Log in as each in two different browsers to see live collaboration.
EOF
