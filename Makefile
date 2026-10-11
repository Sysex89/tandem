COMPOSE := docker compose -f srcs/docker-compose.yml

FLAGS   := --remove-orphans

# Project and volume names
PROJECT_NAME   := srcs

# Frontend Build Artifacts (Safe to delete, just re-builds next time)
FRONTEND_CACHE_VOLUMES := \
	$(PROJECT_NAME)_frontend_nuxt_hidden \
	$(PROJECT_NAME)_frontend_output_hidden

# Dependency Volumes (Safe to delete, just re-installs next time)
MODULE_VOLUMES := \
	$(PROJECT_NAME)_frontend_node_modules \
	$(PROJECT_NAME)_auth_node_modules

# Refuses to start an existing installation on a new, empty Postgres 18 volume while
# its data is still in the Postgres 16 one (see scripts/upgrade-postgres.sh).
check-db:
	@if docker volume inspect mycelium_db >/dev/null 2>&1 && ! docker volume inspect tandem_pg18 >/dev/null 2>&1; then \
		echo "Your database is still on Postgres 16 (volume mycelium_db)."; \
		echo "Run 'make upgrade-postgres' first: it copies it to Postgres 18 and keeps the old volume as a backup."; \
		exit 1; \
	fi

upgrade-postgres:
	@./scripts/upgrade-postgres.sh

# Tear down first so volumes are not still mounted by running containers.
all: check-db
	$(COMPOSE) down $(FLAGS)
	@$(MAKE) up

up: check-db
	$(COMPOSE) up -d --build $(FLAGS)

live: check-db
	$(COMPOSE) up --build

down:
	$(COMPOSE) down $(FLAGS)
	@$(MAKE) cleanv

clean:
	$(COMPOSE) down --rmi all $(FLAGS)
	@$(MAKE) cleanv

# Removes all artifacts and dev containers, does not remove the notes databases
cleanv:
	@echo "Removing only node_modules volumes..."
	@-docker volume rm $(MODULE_VOLUMES) 2>/dev/null
	@echo "Cleaning frontend build cache..."
	@-docker volume rm $(FRONTEND_CACHE_VOLUMES) 2>/dev/null
	@docker image prune -f

# Destructive: will destroy databases, both notes and users
fclean:
	@echo "Removing all volumes..."
	$(COMPOSE) down -v --rmi all $(FLAGS)

# Nuclear: removes all volumes ever created by this project (any naming scheme)
wipe: fclean
	@echo "Removing orphaned project volumes..."
	@docker volume ls -q | grep -E '^(srcs_|mycelium_|tandem_|db$$)' | xargs docker volume rm 2>/dev/null; true
	@echo "Done."

re: clean up

# One-command local demo: generates .env and secrets if missing, starts the
# stack and seeds two users sharing a note. GitHub login is disabled.
demo: check-db
	@./scripts/demo-setup.sh
	$(COMPOSE) up -d --build $(FLAGS)
	@./scripts/demo-seed.sh

# End-to-end tests against the running stack (start it with `make demo`).
e2e:
	cd tests/e2e && pnpm install --frozen-lockfile && pnpm exec playwright test

# Notes service tests (needs cargo). #[sqlx::test] creates a database per test in a
# throwaway Postgres container, which is removed afterwards.
test-notes:
	@docker run -d --rm --name tandem-test-pg -e POSTGRES_PASSWORD=test -p 55432:5432 postgres:18-alpine >/dev/null
	@until docker exec tandem-test-pg pg_isready -h localhost -U postgres >/dev/null 2>&1; do sleep 1; done
	@cd srcs/requirements/notes && DATABASE_URL=postgres://postgres:test@localhost:55432/postgres cargo test; \
		status=$$?; docker stop tandem-test-pg >/dev/null; exit $$status

# Collaboration load test against a running stack; see DOCS/explanation/load-test.md
load:
	cd tests/load && pnpm install --frozen-lockfile && node collab-load.mjs --clients 2,10,25,50

# Hosted demo: pulls the GHCR images on the server named in deploy/.env.deploy
# (see DOCS/howto/deploy-the-demo.md).
deploy:
	@./deploy/deploy.sh

dev: check-db
	docker compose -f srcs/docker-compose-dev.yml up -d --build $(FLAGS)

# ex: make rebuild service=frontend
rebuild:
	$(COMPOSE) up -d --build --no-deps $(service)

logs:
	$(COMPOSE) logs -f $(service)

getlogs:
	$(COMPOSE) logs > all-docker-logs-$(shell date +%Y-%m-%d_%H-%M-%S).txt 2>&1

.PHONY: all check-db upgrade-postgres up live down clean cleanv fclean wipe re demo e2e test-notes load deploy dev rebuild logs getlogs
