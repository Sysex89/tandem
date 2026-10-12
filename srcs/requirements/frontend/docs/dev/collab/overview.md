## The collaborative editing workflow

Every collaborative editing session follows the same path: **connect → load → sync → edit → persist → disconnect**.

The backend is split into two services:
- **notes** (Rust/Axum, port 3003): REST API for note metadata, sharing and slugs
- **editor** (Rust/Axum, port 3004): WebSocket service for real-time CRDT sync via the y-sync v1 protocol

Both use the same PostgreSQL database. yrs is the Rust port of Yjs. For the frontend side, see [tiptap-yjs-integration.md](./tiptap-yjs-integration.md); for cursors, [live-cursors.md](./live-cursors.md).

### Connect

A client opens a WebSocket to `/ws/{id}`. nginx first runs `auth_request` against the auth service and injects the verified `X-User-Id`. Before upgrading, the editor service resolves the user's access with `db::access_level`: owner or Edit share → read-write, View share → read-only, nothing → 403.

It then looks up the note's room in the in-memory rooms map (`DashMap`). If a room exists, the client joins it; if not, a new room is created by loading the document from PostgreSQL.

### Load

Loading only happens when the first client connects to a note. The `note_states.state_vector` column (`BYTEA`; despite the name it holds the full document encoded as a Yjs update) is applied to a new yrs `Doc`, which becomes the room's in-memory truth.

### Sync

The editor service speaks the **y-sync v1 protocol** (the same as y-websocket). On connect:
1. Server and client exchange state vectors and missing updates via the standard SyncStep1/SyncStep2 handshake
2. The server sends the current awareness states (other users' cursors)

The frontend uses `y-websocket`'s `WebsocketProvider`, which speaks the same protocol natively.

### Edit

Each edit produces a Yjs update (binary diff). The server applies it to the in-memory Doc, marks the room dirty and broadcasts it to the other clients. Updates from read-only clients are dropped; they still sync and send awareness. Access is re-checked every 15 s per connection, so a revoked share or a role change applies to open sockets.

Awareness updates (cursor positions, selections) are ephemeral: they are relayed as received and never touch the database.

Each client has a bounded send queue. A client that falls too far behind is disconnected; y-websocket reconnects and resyncs it.

### Persist

The full document is written back to `note_states`:
- every 5 s while the room is dirty (a background task per room),
- when the last client leaves,
- on shutdown: SIGTERM closes every socket with code 1012 so clients reconnect, and rooms are saved as they empty.

After each save, the editor pushes the title (the Yjs `title` text) to the notes service's `PUT /api/notes/{id}` if it changed. The editor is the only writer of `notes.title`; the frontend never persists titles itself.

### Disconnect

When a client disconnects it is removed from the room. If it was the last one, the room is saved and removed from memory.

### REST API (notes service)

| Endpoint | Purpose |
|---|---|
| `GET /api/notes` | List the user's notes (id, title, slug, timestamps) |
| `GET /api/notes/{id}` | Get a single note |
| `POST /api/notes` | Create a note with an initial title (also creates its `note_states` row) |
| `PUT /api/notes/{id}` | Update the title and slug (called by the editor service) |
| `DELETE /api/notes/{id}` | Delete a note |

Document content changes only through the WebSocket. The full list, including sharing, is in the [notes service README](../../../../notes/README.md).
