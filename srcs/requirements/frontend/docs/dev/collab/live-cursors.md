# Live Cursors

Live cursors let users see each other's cursor positions and text selections in real time. They use the **Yjs Awareness protocol**, which is separate from the document CRDT sync — cursor data is ephemeral (never persisted) and handled by the y-sync protocol's awareness layer.

---

## How it works

### Awareness vs document sync

| | Document sync | Awareness |
|---|---|---|
| **What it carries** | Text insertions, deletions, formatting | Cursor position, selection range, user name, color |
| **Persisted?** | Yes, saved to `note_states` (every 5 s while changed, and when the room empties) | No, only held in memory while the room is active |
| **Lifetime** | Permanent (survives restart via Postgres) | Ephemeral (lost when last client disconnects) |

Both travel over the same WebSocket connection, multiplexed by the y-sync v1 protocol.

### Awareness state contents

Each client's awareness state is a JSON object:

```json
{
  "user": { "name": "alice", "color": "#958DF1" },
  "cursor": { "anchor": 42, "head": 42 },
  "selection": { "anchor": 42, "head": 50 }
}
```

These fields are set automatically by `@tiptap/extension-collaboration-caret` whenever the ProseMirror selection changes.

---

## Frontend implementation

| Component / File | Role |
|---|---|
| `NoteEditor.vue` | Configures `CollaborationCaret` with the provider and user info |
| `useCollaboration.ts` | Creates `WebsocketProvider` (from y-websocket), tracks `connectedUsers` count |
| `utils/caretRenderer.ts` | `userColor()` (a stable color per user ID) and the caret element, whose name label fades 2 s after the cursor last moved |
| `base.css` | Styles the cursor caret and username label |

### CollaborationCaret configuration

```typescript
CollaborationCaret.configure({
  provider: prov,  // y-websocket WebsocketProvider — exposes .awareness
  user: {
    name: authStore.user?.loginName ?? t('noteEditor.anonymous'),
    color: userColor(authStore.user?.id),
  },
  render: createCaretRenderer(prov.awareness),
})
```

### Connected users count

```typescript
provider.awareness.on("change", () => {
  connectedUsers.value = provider.awareness.getStates().size;
});
```

Displayed in `NoteEditor.vue` when more than one user is present.

---

## Key design properties

1. **Ephemeral**: Cursor data is never written to the database.
2. **Automatic cleanup**: When a client disconnects, awareness removal is broadcast to remaining clients.
3. **Low latency**: Awareness updates skip the persistence path entirely. The editor service relays each awareness message as received instead of re-encoding the full awareness state, which matters with many clients (see `DOCS/explanation/load-test.md`).
4. **Single WebSocket**: Cursor awareness and document sync share one connection via y-sync protocol multiplexing.
