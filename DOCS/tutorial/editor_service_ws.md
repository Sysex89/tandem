# Editor Service (Real-time Collaboration)

This service manages the real-time collaborative editing sessions for notes using [Yrs](https://github.com/y-crdt/y-crdt) (Rust port of Yjs) and WebSockets. This tutorial walks through how it works and why; the reference (endpoints, environment variables) is in [the editor README](../../srcs/requirements/editor/README.md).

## 🧠 Core Concept: CRDTs & Yjs

The editor does not save "text" directly. It saves a **CRDT** (Conflict-free Replicated Data Type) document: a structure that merges concurrent edits from several people without conflicts.
- **Yrs (Rust)**: Runs on this server. Holds the "source of truth" document in memory.
- **Yjs (JS)**: Runs on the frontend. Syncs with this server.

When you type in the frontend:
1. Yjs generates a binary "update".
2. Update is sent via WebSocket to this service.
3. This service applies the update to its in-memory `Doc`.
4. This service broadcasts the update to all other connected clients.

## 🔄 Data Architecture

### 1. The WebSocket Connection (`/ws/{note_id}`)
- **Endpoint**: `wss://<host>/ws/<uuid>`, through nginx
- **Handler**: `src/ws_handler.rs`
- **Logic**:
    - nginx has already verified the session and set `X-User-Id`; without it the service answers 401.
    - Looks up the user's access to the note (`db::access_level`): owner or Edit share → read-write, View share → read-only, nothing → 403.
    - Upgrades the connection to a WebSocket.
    - Finds the `DocumentRoom` for this `note_id`, or creates it by loading the saved state from Postgres (`db::load_note`).
    - Spawns tasks to handle incoming/outgoing messages for this client.

### 2. In-Memory State (`state.rs`)
- **`AppState`**: Holds a concurrent map (`DashMap`) of active rooms, the database pool and an HTTP client for the notes service.
- **`DocumentRoom`**: Represents one open note. Contains:
    - The Yrs `Doc` (the actual CRDT structure) and the awareness state (cursors).
    - The connected clients, each with a bounded outgoing queue.
    - A `dirty` flag: set when the document changes, cleared when it is saved.

### 3. Database Persistence (`db.rs`)
The service reads and writes the `note_states` table in Postgres.
- **Loading**: When the first user opens a note, we fetch the `state_vector` blob from the DB and apply it to a new Yrs `Doc`.
- **Saving**: Every 5 seconds while the room is dirty, when the last user leaves, and on shutdown, we encode the whole `Doc` and `UPDATE` the row.

> **Note**: We store the *merged result* (the whole document encoded as one Yjs update), not every keystroke, to keep DB size manageable. Despite its name, the `state_vector` column holds that update, not a Yjs state vector.

### 4. Titles (`title.rs`)
The note title is a `title` text inside the same Yjs document, so renames sync live like the body. After each save, the editor pushes a changed title to the notes service (`PUT /api/notes/{id}`, as the owner) so the sidebar and the slug follow. It is the only writer of `notes.title`.

## 📦 Key Files
- `src/main.rs`: Entry point. Initializes metrics, the DB pool, logging (tracing) and the Router; handles graceful shutdown.
- `src/ws_handler.rs`: WebSocket lifecycle (access check, connect, broadcast, disconnect, save on last leave).
- `src/sync.rs`: Decodes y-sync messages and applies them to the document or the awareness.
- `src/db.rs`: Access check, and loading/saving the CRDT blobs.
- `src/title.rs`: Pushes the shared title to the notes service.
- `src/shutdown.rs`: On SIGTERM, closes every socket with 1012 so clients reconnect, and saves every open room.
- `src/metrics.rs`: Prometheus metrics (`editor_*`).
- `src/models.rs`: `Access` (read-only / read-write) and the row struct.

## 🐛 Debugging
Logs are output via `tracing`; the default filter already includes `editor=debug`. Override it with `RUST_LOG`.
```bash
# Check logs in docker
# -f: --follow
docker logs -f editor
```


# 🔬 Deep Dive: Implementation Details

This section breaks down the actual Rust code to help you recreate the service.

## 1. The WebSocket Handshake ( `src/ws_handler.rs` )

The entry point is a standard HTTP GET request. Axum upgrades it to a WebSocket if the headers are correct.

### 1.1 What is a WebSocket?
Normally, the web runs on **HTTP**: the user asks (Request), and the server answers (Response). After the answer, the connection closes.

A **WebSocket** starts as an HTTP Request but says: *"Hey, can we keep this line open and talk freely?"*

If the server agrees, it responds with **101 Switching Protocols**. The connection stays open. Now, both the client and server can send data at any time without asking first. This is called **Full-Duplex** communication.

### 1.2 "Upgrading" the Connection
The "Handshake" is that initial HTTP request to upgrade the protocol. In Axum (our web framework), this is handled by the `WebSocketUpgrade` extractor.

### 1.3 Annotated Code: The "Extractor" Magic
This is the most confusing part of Rust web frameworks (Axum), but also the most powerful. 

You might wonder: *How does declaring a variable check headers?*

The answer is **Dependency Injection**. Axum does not run your function immediately.
1. Axum looks at the type of arguments you asked for (`WebSocketUpgrade`).
2. It runs a hidden pre-check logic associated with that type (checking headers).
3. **If headers are missing**: Axum stops. It returns `400 Bad Request` to the user. Your function **is never called**.
4. **If headers are good**: Axum builds the `ws` object and finally runs your function.

#### Who Calls `ws_route`? (The Trigger)

It is called **milliseconds AFTER the page loads**, when your JavaScript specifically asks to "connect".

**The Sequence:**
1.  **User visits page:** Browser receives the server-rendered page and its JavaScript (served by nginx and the Nuxt frontend; the **editor service sleeps**).
2.  **JS starts:** The Vue app initializes in the browser.
3.  **JS executes:** The line `new WebsocketProvider(...)` runs.
4.  **Browser Request:** The browser sends a background request to `wss://localhost:8443/ws/<note id>`. nginx checks the session (`auth_request`) and forwards it with `X-User-Id` set.
5.  **Server Match:** Axum sees the request to `/ws/...`, matches it to your `.route`, and **NOW** calls `ws_route`.
```rust
// The route definition in main.rs
.route("/ws/{id}", get(ws_handler::ws_route))

// the handler in ws_handler.rs (simplified)
pub async fn ws_route(
    // 1. THE GUARD (Extractor)
    // By listing this type, you force Axum to validate the request first.
    // Axum internally calls `WebSocketUpgrade::from_request(req)`.
    // It verifies:
    //   - "Connection: Upgrade"
    //   - "Upgrade: websocket"
    //   - "Sec-WebSocket-Key: <random_key>"
    // Only if ALL exist, this `ws` variable is created.
    ws: WebSocketUpgrade,

    // 2. PATH EXTRACTOR
    // Axum parses the URL "/ws/123e4567..."
    // and converts the string ID into a Uuid type for you.
    Path(note_id): Path<Uuid>,

    // 3. STATE EXTRACTOR
    // This connects back to `.with_state(app_state)` in main.rs.
    // It gives us the database pool and the map of active rooms.
    // "Dependency injection"
    State(state): State<Arc<AppState>>,

    // 4. HEADERS
    // nginx put the verified user ID here.
    headers: HeaderMap,
) -> Response {
    // 5. AUTHORIZE before upgrading: a plain HTTP error is easy for the client to read,
    // while a socket that opens and immediately closes is not.
    let Some(user_id) = user_id_from(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(access) = db::access_level(&state.pool, note_id, user_id).await else {
        return StatusCode::FORBIDDEN.into_response();
    };

    // 6. Finalize the handshake.
    // We return a Response that sends "HTTP 101 Switching Protocols" back to the client,
    // AND we provide a callback (`handle_socket`) that will now run
    // in a background task to manage the open connection.
    ws.on_upgrade(move |socket| handle_socket(socket, note_id, state, user_id, access))
}
```

### 1.4 Where does the Request come from?
It comes from the **Frontend** (your browser).

In `srcs/requirements/frontend/app/composables/useCollaboration.ts`, we have this TypeScript code:

```typescript
import { WebsocketProvider } from "y-websocket";

// ... inside a function ...
const protocol = location.protocol === "https:" ? "wss:" : "ws:";
const wsUrl = `${protocol}//${location.host}/ws`; // e.g. "wss://localhost:8443/ws"

// THIS LINE triggers the request!
const newProvider = new WebsocketProvider(wsUrl, id, newYdoc);
```

When this line runs in Chrome/Firefox:
1.  **Our Code calls** `new WebsocketProvider(...)`. This is a helper class from the `y-websocket` library.
2.  **The Library calls** `new WebSocket(url)` internally. This is the official Browser API.
3.  **The Browser itself** automatically constructs an HTTP GET request.
4.  **The Browser itself** adds the `Upgrade: websocket`, `Connection: Upgrade`, and `Sec-WebSocket-Key` headers.
5.  The request flies across the network to nginx, which checks the session cookie with the auth service and then proxies it to the Rust backend.

So, "what puts those headers in there?" -> **The Browser (Chrome/Firefox/Safari)**, following the [RFC 6455](https://tools.ietf.org/html/rfc6455) standard. You don't write them manually.

MDN Documentation: [MDN - WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)

### 1.5 WebSocket vs. Streams (Why not use a Stream?)
You might hear about `WebSocketStream` (raw byte streams) or just TCP Streams.

*   **We use `WebSocket` (Message-Based):**
    *   **Concept:** "Here is a full packet."
    *   **Why:** Yjs updates are distinct, self-contained binary blobs. We need the *entire* update to apply it to the document. The WebSocket protocol handles the "framing" (knowing where a message starts and ends) for us.
    *   **Analogy:** Receiving letters. You wait for the postman, he hands you a full envelope. You open it and read it.

*   **When to use a raw Stream:**
    *   **Concept:** "Here is a continuous flow of bytes."
    *   **Why:** If you were streaming live **Audio/Video**, or transferring a massive 10GB file. You would handle the raw bytes yourself to manage buffering (backpressure) precisely.
    *   **Analogy:** A water hose. You just get water, and you have to decide when to stop drinking.


## 2. State Management with DashMap (`src/state.rs`)
We need to share document state across many WebSocket connections (tasks running on different threads).
*   **DashMap**: A concurrent HashMap, sharded so different rooms don't contend for one global lock.
*   **Arc & RwLock**: To safely share the Yrs Document and Awareness between tasks.
*   **Bounded channels**: Each client gets an `mpsc` queue of `CLIENT_QUEUE` (2048) messages. Broadcasting uses `try_send`, so one slow client can never block the others or grow memory without limit.

```rust
pub struct Client {
    pub tx: Sender<Message>,   // this client's outgoing queue
    pub evict: Arc<Notify>,    // fired when the queue is full
}

pub struct DocumentRoom {
    // The "Source of Truth" CRDT document
    pub doc: Arc<RwLock<Doc>>,
    // Ephemeral state (cursors, selections) - not saved to DB
    pub awareness: Arc<RwLock<Awareness>>,
    // Active connections to broadcast updates to
    pub clients: DashMap<usize, Client>,
    // Changed since the last save?
    pub dirty: AtomicBool,
    // Last title pushed to the notes service
    pub saved_title: Mutex<String>,
}

pub struct AppState {
    pub pool: PgPool,
    // Map: NoteID -> Active Room
    pub rooms: DashMap<Uuid, Arc<DocumentRoom>>,
    pub http: reqwest::Client,
    pub notes_url: String,
    pub shutting_down: AtomicBool,
}
```

## 3. The Sync Loop (`ws_handler.rs` & `sync.rs`)
This is the heartbeat of the service.
1.  **Receive** a binary message from Client A.
2.  **Apply** it to the local `Doc` (or to the awareness, for cursor messages). Document updates from a read-only client are dropped here.
3.  **Broadcast** the result to Clients B, C, D.

```rust
// In handle_socket (ws_handler.rs), simplified
match sync::process_binary_message(&bytes, &room.doc, &room.awareness, &room.dirty, access).await {
    Ok(response) if !response.is_empty() => {
        let mut too_slow = Vec::new();
        for client in room.clients.iter() {
            if *client.key() != my_client_id
                && let Err(TrySendError::Full(_)) = client.value().tx.try_send(msg(response.clone()))
            {
                too_slow.push(*client.key());
            }
        }
        // Disconnect clients whose queue is full; y-websocket reconnects them
        // and resyncs the whole document, which is cheaper than buffering forever.
        for id in too_slow { evict(id) }
    }
    _ => {}
}
```

The same loop runs a 15-second timer that re-checks the user's access, so a revoked share closes the socket and a role change (Edit ↔ View) applies without reconnecting.

## 4. Persistence Strategy (`ws_handler.rs` & `db.rs`)
We **load lazily** and **save in the background**:

*   **Load**: When the *first* user connects, we fetch the blob from Postgres.
*   **Save periodically**: Each room has a background task that wakes every 5 seconds and saves if `dirty` is set. A crash loses at most a few seconds of typing.
*   **Save on last leave**: When the *last* user disconnects, we save and remove the room from memory.
*   **Save on shutdown**: On SIGTERM (`docker stop`), `shutdown.rs` closes every socket with code 1012 ("service restart"), so the clients reconnect to the next instance; each room saves as it empties, and any room still open after 5 seconds is saved directly.

```rust
// End of handle_socket (ws_handler.rs), simplified
room.clients.remove(&client_id);

// If the room is empty, no one is editing -> Save and Cleanup
if room.clients.is_empty() {
    let doc = room.doc.read().await;
    db::save_note(&state.pool, note_id, &doc, "last_client").await;
    drop(doc);
    title::sync_title(&state, &room, note_id).await;

    // Remove from memory to save RAM
    state.rooms.remove(&note_id);
}
```

## 5. The Database Schema
Very simple. We store the document as a single binary column. Cracking open the CRDT format in SQL is hard/impossible, so we treat it as a blob. The table belongs to the notes service's migrations (`notes/migrations/`); the editor only reads and updates rows.

```sql
CREATE TABLE note_states (
    note_id UUID PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
    state_vector BYTEA NOT NULL,  -- the whole document, encoded as one Yjs update
    last_saved_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 🧠 Topics for Further Study
To fully master this service, you should look into:
1.  **Yjs Protocol**: How the binary update format works (V1 vs V2), and the y-sync message types (SyncStep1, SyncStep2, Update, Awareness).
2.  **Tokio Channels (`mpsc`)**: Bounded vs unbounded queues, and why `try_send` matters for a broadcast.
3.  **Axum Extractors**: How `Path`, `State`, `HeaderMap` and `WebSocketUpgrade` are pulled from the request.
4.  **Graceful shutdown**: Signals, WebSocket close codes, and draining work within Docker's stop timeout.
5.  **Load behaviour**: [The load test write-up](../explanation/load-test.md) shows what broke with 50 concurrent typists and how it was fixed.
