# Origins: 42 Berlin ft_transcendence

Mycelium Notes started as the final group project of the 42 Berlin core curriculum (ft_transcendence). This is the original defense write-up, kept as it was at evaluation time. The "Database schema" section is out of date; see [reference/database-schema.md](reference/database-schema.md) for the current schema.


## Team information

Ahmed Diler - Developer

Maarten Hoff - Technical Lead, Developer

Masha Yakovenko - Product Owner, Developer

Pavlos Vasilantonakis - Project Manager

Grace Mullin - Developer

## **IV. Project management**

* **Organization**: Features were decided early. The main sync-time was a weekly in-person meeting with additional video calls between subsets of the team. Team members expressed interest in specific subject modules early and were encouraged to take ownership. The product owner was tasked with mantaining a forward momentum, the technical lead was asked to solve any ties in relation to tech and the project lead attempted to partially disengage from the everyday coding in order to keep the rest of the team motivated.

* **Tools**: 
    Github Issues for matching commits to issues
    Slack for everyday communication
    Github projects for the gratification of the Project Manager

## **V. Technical stack**

* **Frontend**: Nuxt 4 / Vue 3 with TailwindCSS, PrimeVue, and Pinia. TipTap provides the rich-text editor; Yjs and y-websocket handle real-time collaborative editing via CRDTs. Framework choice satisfies the 42 curriculum requirement.
* **Auth service** (TypeScript): Fastify 5 with Drizzle ORM. Handles OAuth (GitHub, Google, 42) and local password auth (Argon2). Session-based authentication with cookie tokens. Exposes Prometheus metrics via `prom-client`.
* **Notes service** (Rust): Axum + sqlx. CRUD API for note metadata. Chosen for low overhead on a high-frequency path.
* **Editor service** (Rust): Axum with WebSocket support + Yrs/y-sync. Persists Yjs CRDT document state to PostgreSQL. Handles real-time sync between collaborating clients.
* **Database**: PostgreSQL 16. Two logical databases in a single instance — one for auth (users, accounts, sessions), one for notes and document state. Relational model fits the structured data; UUID primary keys throughout.
* **Infrastructure**: Docker Compose orchestrates 10 services behind an Nginx reverse proxy. Nginx gates `/api/notes` and `/ws/` routes through an `auth_request` subrequest to the auth service. Prometheus scrapes all application and infrastructure targets; Grafana provides pre-provisioned dashboards. Exporters for Nginx and PostgreSQL round out observability.

## **VI. Database schema**

Two logical databases in a single PostgreSQL 16 instance.

**Auth database** (Drizzle ORM, migrations in `auth/drizzle/`):

| Table | Columns | Notes |
| --- | --- | --- |
| `users` | `id` (UUID PK), `email` (unique), `login_name` (unique, not null), `image_url`, `role` (enum: user/admin), `status` (enum: active/blocked/suspended), `created_at` | Central identity table |
| `accounts` | `id` (UUID PK), `user_id` (FK → users, cascade), `provider` (enum: github/local/google/42), `provider_account_id`, `password_hash` | Unique on (provider, provider_account_id). One user can have multiple provider accounts |
| `sessions` | `id` (UUID PK), `user_id` (FK → users, cascade), `token` (UUID, unique), `expires_at`, `user_agent`, `ip_address` | Cookie-based session tracking |

**Main database** (SQL init in `postgres/init/02-init.sql` at the time; now sqlx migrations in `notes/migrations/`):

| Table | Columns | Notes |
| --- | --- | --- |
| `notes` | `id` (UUID PK), `title` (varchar 255), `owner_id` (UUID), `created_at`, `updated_at` | Note metadata |
| `note_states` | `note_id` (UUID PK, FK → notes, cascade), `state_vector` (BYTEA), `last_saved_at` | Persisted Yjs CRDT state, 1:1 with `notes` |
| `share` | `id` (UUID PK), `note_id` (UUID, FK → notes, cascade), `guest_id` (UUID), `role` (varchar 50, default 'View'), `created_at`, `updated_at` | Note sharing; unique (note_id, guest_id) |


**Key relations**: one user → many accounts (multi-provider auth), one user → one session, one note → one note_state, one note -> many share. All foreign keys cascade on delete.

## **VII. Features list**

## **VIII. Modules & points**

Calculate your path to the 14-point minimum.

| Module | Category | Type | Points |
| --- | --- | --- | --- |
| Frameworks (Vue + Axum) | Web | Major | 2 | 
| Microservices | DevOps | Major | 2 |
| Monitoring System with Grafana and Prometheus | DevOps | Major | 2 |
| RAG System | AI | Major | 2 |
| LLM System Interface | AI | Major | 2 |
| Web-socket communication | Web | Major | 2 |
| Real-time collaboration | Web | Minor | 1 |
| Server-Side Rendering | Web | Minor | 1 |
| Support for multiple languages | Accesibility | Minor | 1 |
| Support for rtl | Accesibility | Minor | 1 |
| Support for 2 additional browsers [Firefox, Konqueror] | Accesibility | Minor | 1 |
| OAuth Authentication | User Management | Minor | 1 |
|ORM system | Web | Minor | 1 |
|Standard user management|User Management | Major | 2|
| GDPR compliance | Data | Minor | 1 |
| **Total estimated** | | | **18** |

## **IX. Individual contributions**

Frameworks/Microservices: Ahmed, Maarten, Masha

Web-Sockets: Ahmed, Maarten, Masha, Pavlos

Authentication: Masha

Frontend and SSR: Masha, Ahmed, Grace

Notes API: Maarten, Masha

Monitoring/Browser Testing: Pavlos

RAG/LLM: Maarten, Ahmed

Localization: Grace

## **X. AI usage**
Pavlos: Used AI to research the pros and cons of different stacks, for detecting code smells, for changes that needed multi-file editing to make sure all contact points were adequately worked on and for the creation of tutorials and learning material for the project
Maarten: Used AI for research, brainstorming, structure/idea validation, writing repetitive code, and double-checking documentation.
Masha: Used AI for research, writing repetetive code, assistance with learning frameworks like Fastify, tutorials, assistance with tricky styling issues, keeping track of refactors and changes that needed multifile editing, particularly for UI bugs.

## **Future Features**
What in-theory would be an obvious next step:

    user management with accounts
    
    extended dashboards
    
    sentiment analysis and semantic tagging
    
    friends and networks (?)
    
    backlinks

---
