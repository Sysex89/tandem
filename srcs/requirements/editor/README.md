# Editor Service

Collaborative note editing backend built with Rust, Axum, and Yjs (via `yrs` / `y-sync`). Exposes a WebSocket endpoint per note, maintains an in-memory CRDT document for each active room, and persists state to PostgreSQL.

---

## Endpoint

| Method | Path | Purpose |
| :--- | :--- | :--- |
| **GET** | `/ws/{id}` | WebSocket upgrade for collaborative editing on note `id` (UUID). |
| **GET** | `/metrics` | Prometheus metrics (`editor_*`). Internal only: nginx does not route it. |

Authentication is handled by Nginx: the `X-User-Id` header (UUID) must be set by the reverse proxy (401 without it). Before upgrading, the service resolves the user's access from the `notes` / `share` tables: owner or Edit share → read-write, View share → read-only, nothing → 403.

---

## Environment Variables

| Variable | Default | Description |
| :--- | :--- | :--- |
| `DB_USER` | — | PostgreSQL username. |
| `DB_PASSWORD` | — | PostgreSQL password. |
| `DB_NAME` | — | PostgreSQL database name. |
| `DB_HOST` | `postgres` | PostgreSQL hostname. |
| `PORT` | `3004` | Port the service listens on. |
| `NOTES_URL` | `http://notes:3003` | Notes service, for pushing title changes. |
| `RUST_LOG` | `editor=debug,…` | `tracing` log filter. |

---

## How It Works

On click of a note title the frontend opens a WebSocket connection via Nginx to `/ws/{id}`.

When the first client connects, the service loads the persisted binary state from `note_states` in Postgres, applies it to a new `yrs::Doc`, and creates a `DocumentRoom` in a `DashMap` (thread-safe concurrent map). Subsequent clients joining the same note reuse the existing room without a DB query.

Each client receives the current state vector and awareness info on connect. Live edits arrive as binary y-sync messages, are applied to the in-memory Doc, and broadcast to all other clients in the room. Each client has a bounded send queue (`CLIENT_QUEUE`); a client that falls that far behind is disconnected and resyncs when y-websocket reconnects.

### Access

Document updates from read-only clients are dropped (they still sync and send awareness). Access is re-checked every 15 s per connection, so a revoked share closes the socket and a role change applies without reconnecting. The frontend's read-only editor is only UX; this is the enforcement.

### Persistence

A background task runs a save loop (every 5 seconds) for each active room. If the document has been modified (`dirty` flag), it encodes the full CRDT state and writes it back to `note_states`. When the last client disconnects, a final save is triggered and the room is removed from memory.

On SIGTERM the service stops accepting connections and closes every socket with code 1012 (service restart), so clients reconnect to the next instance. Rooms save as they empty; any still open after 5 s are saved directly (`src/shutdown.rs`).

### Titles

The note title is the `title` text in the Yjs document. After each save, a changed title is pushed to the notes service (`PUT /api/notes/{id}` as the owner), which also regenerates the slug. The editor is the only writer of `notes.title`.

### Awareness

Cursor positions, user colors, and selection state are relayed between clients via y-sync awareness messages. These are held in memory for new joiners but are never applied to the CRDT document and never persisted.

---

## Dependencies

- [axum](https://github.com/tokio-rs/axum) (WebSocket + HTTP)
- [tokio](https://tokio.rs/) (async runtime)
- [sqlx](https://github.com/launchbadge/sqlx) (PostgreSQL)
- [yrs](https://github.com/y-crdt/y-crdt) / [y-sync](https://github.com/y-crdt/y-crdt) (Yjs CRDT)
- [dashmap](https://github.com/xacrimon/dashmap) (concurrent room map)
- [reqwest](https://github.com/seanmonstar/reqwest) (title pushes to the notes service)
- [prometheus](https://github.com/tikv/rust-prometheus) (metrics)
- [tracing](https://github.com/tokio-rs/tracing) (structured logging)

A longer walkthrough of the code is in [DOCS/tutorial/editor_service_ws.md](../../../DOCS/tutorial/editor_service_ws.md).
