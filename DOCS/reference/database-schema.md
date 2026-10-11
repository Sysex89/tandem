# Database schema

Two logical databases in one PostgreSQL 16 instance. User IDs cross database boundaries without foreign keys, so cleanup on account deletion is done explicitly by the auth service (it calls the notes service).

## Auth database

Owned by the `auth` service. Schema in `srcs/requirements/auth/src/db/schema.ts`, migrations in `srcs/requirements/auth/drizzle/` (applied at startup).

| Table | Columns | Notes |
| --- | --- | --- |
| `users` | `id` (UUID PK), `email` (unique), `login_name` (unique, not null), `image_url`, `role` (enum: user/admin), `status` (enum: active/blocked/suspended), `created_at` | Central identity table |
| `accounts` | `id` (UUID PK), `user_id` (FK → users, cascade), `provider` (enum: github/local/google/42), `provider_account_id`, `password_hash` | Unique on (provider, provider_account_id). One user can link several providers |
| `sessions` | `id` (UUID PK), `user_id` (FK → users, cascade), `token` (UUID, unique), `expires_at`, `user_agent`, `ip_address` | Cookie-based sessions |

## Main database

Shared by `notes` (metadata, sharing) and `editor` (document state). The schema is owned by the notes service as sqlx migrations in `srcs/requirements/notes/migrations/`, applied at notes startup (on fresh and existing volumes alike) and recorded in `_sqlx_migrations`.

| Table | Columns | Notes |
| --- | --- | --- |
| `notes` | `id` (UUID PK), `title` (varchar 255, default 'Untitled'), `owner_id` (UUID), `owner_url` (varchar 255), `created_at`, `updated_at` | Note metadata; `owner_url` is the slug used by `/api/notes/u/{slug}`, unique per owner (`notes_owner_slug_key`) |
| `note_states` | `note_id` (UUID PK, FK → notes, cascade), `state_vector` (BYTEA), `last_saved_at` | Persisted Yjs document, 1:1 with `notes` |
| `share` | `id` (UUID PK), `note_id` (FK → notes, cascade), `guest_id` (UUID), `access_role` (enum `access_type`: View/Edit/Owner, default View), `created_at`, `updated_at` | Unique on (note_id, guest_id), nulls not distinct (one public link per note); indexed on `guest_id` |

