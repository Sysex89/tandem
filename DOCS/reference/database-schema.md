# Database schema

Three logical databases across two PostgreSQL 16 instances. User IDs cross database boundaries without foreign keys, so cleanup on account deletion is done explicitly by the auth service (it calls notes and ai-ingest).

## Auth database

Owned by the `auth` service. Schema in `srcs/requirements/auth/src/db/schema.ts`, migrations in `srcs/requirements/auth/drizzle/` (applied at startup).

| Table | Columns | Notes |
| --- | --- | --- |
| `users` | `id` (UUID PK), `email` (unique), `login_name` (unique, not null), `image_url`, `role` (enum: user/admin), `status` (enum: active/blocked/suspended), `created_at` | Central identity table |
| `accounts` | `id` (UUID PK), `user_id` (FK → users, cascade), `provider` (enum: github/local/google/42), `provider_account_id`, `password_hash` | Unique on (provider, provider_account_id). One user can link several providers |
| `sessions` | `id` (UUID PK), `user_id` (FK → users, cascade), `token` (UUID, unique), `expires_at`, `user_agent`, `ip_address` | Cookie-based sessions |

## Main database

Shared by `notes` (metadata, sharing) and `editor` (document state). Schema in `srcs/requirements/postgres/init/02-init.sql`; it only runs when the `mycelium_db` volume is first created.

| Table | Columns | Notes |
| --- | --- | --- |
| `notes` | `id` (UUID PK), `title` (varchar 255, default 'Untitled'), `owner_id` (UUID), `owner_url` (varchar 255), `created_at`, `updated_at` | Note metadata; `owner_url` is the slug used by `/api/notes/u/{slug}` |
| `note_states` | `note_id` (UUID PK, FK → notes, cascade), `state_vector` (BYTEA), `last_saved_at` | Persisted Yjs document, 1:1 with `notes` |
| `share` | `id` (UUID PK), `note_id` (FK → notes, cascade), `guest_id` (UUID), `access_role` (enum `access_type`: View/Edit/Owner, default View), `created_at`, `updated_at` | Unique on (note_id, guest_id), nulls not distinct |


## Vector database

Separate pgvector instance (`vector-db`), written by `ai-ingest` and read by `ai-rag`. Schema in `srcs/requirements/vector-db/init/01-init.sql`.

| Table | Columns | Notes |
| --- | --- | --- |
| `embeddings` | `id` (UUID PK), `note_id`, `user_id`, `content` (text chunk), `embedding` (`vector(1024)`) | No index yet; similarity search is a sequential scan |
