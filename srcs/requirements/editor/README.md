# Editor Service

Collaborative note editing backend built with Rust, Axum, and Yjs (via `yrs` / `y-sync`). Exposes a WebSocket endpoint per note, maintains an in-memory CRDT document for each active room, and persists state to PostgreSQL.

---

## Endpoint

| Method | Path | Purpose |
| :--- | :--- | :--- |
| **GET** | `/ws/{id}` | WebSocket upgrade for collaborative editing on note `id` (UUID). |

Authentication is handled by Nginx: the `X-User-Id` header (UUID) must be set by the reverse proxy. The service verifies ownership or share access against the `notes` / `share` tables before upgrading the connection.

---

## Environment Variables

| Variable | Default | Description |
| :--- | :--- | :--- |
| `DB_USER` | — | PostgreSQL username. |
| `DB_PASSWORD` | — | PostgreSQL password. |
| `DB_NAME` | — | PostgreSQL database name. |
| `DB_HOST` | `postgres` | PostgreSQL hostname. |
| `PORT` | `3004` | Port the service listens on. |

---

## How It Works

On click of a note title the frontend opens a WebSocket connection via Nginx to `/ws/{id}`.

When the first client connects, the service loads the persisted binary state from `note_states` in Postgres, applies it to a new `yrs::Doc`, and creates a `DocumentRoom` in a `DashMap` (thread-safe concurrent map). Subsequent clients joining the same note reuse the existing room without a DB query.

Each client receives the current state vector and awareness info on connect. Live edits arrive as binary y-sync messages, are applied to the in-memory Doc, and broadcast to all other clients in the room.

### Persistence

A background task runs a debounced save loop (every 5 seconds) for each active room. If the document has been modified (`dirty` flag), it encodes the full CRDT state and writes it back to `note_states`. When the last client disconnects, a final save is triggered and the room is removed from memory.

### Awareness

Cursor positions, user colors, and selection state are relayed between clients via y-sync awareness messages. These are held in memory for new joiners but are never applied to the CRDT document and never persisted.

---

## Dependencies

- [axum](https://github.com/tokio-rs/axum) (WebSocket + HTTP)
- [tokio](https://tokio.rs/) (async runtime)
- [sqlx](https://github.com/launchbadge/sqlx) (PostgreSQL)
- [yrs](https://github.com/y-crdt/y-crdt) / [y-sync](https://github.com/y-crdt/y-crdt) (Yjs CRDT)
- [dashmap](https://github.com/xacrimon/dashmap) (concurrent room map)
- [tracing](https://github.com/tokio-rs/tracing) (structured logging)
