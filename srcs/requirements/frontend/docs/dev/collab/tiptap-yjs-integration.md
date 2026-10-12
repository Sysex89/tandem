# Tiptap + Yjs Frontend Integration

For the backend workflow, see [overview.md](./overview.md). For the live cursor system, see [live-cursors.md](./live-cursors.md).

---

## Package Dependencies

`@tiptap/vue-3`, `@tiptap/starter-kit`, `@tiptap/extension-collaboration`, `@tiptap/extension-collaboration-caret`, `yjs` and `y-websocket` (versions in `package.json`).

`y-websocket` provides the `WebsocketProvider` that speaks the y-sync v1 protocol, matching the backend's editor service.

---

## Architecture: 2-file system

```
app/composables/useCollaboration.ts    app/components/notes/NoteEditor.vue
(Y.Doc, WebsocketProvider,             (Tiptap Editor with
 shared title, lifecycle)               Collaboration extensions)
```

### useCollaboration composable

- Accepts `noteId: () => string` getter — reactive to note switches
- Uses `watch` + pending/current pattern: new connection syncs in background, swaps after first sync (no blank flash)
- Uses `shallowRef` for Y.Doc and WebsocketProvider (avoids deep Vue proxying)
- WebSocket URL: `ws(s)://{host}/ws` with noteId as room name
- Sync detection: `provider.on("sync", (isSynced) => ...)` — y-websocket's native sync event

### NoteEditor component

- Uses `watchEffect` to create/destroy Tiptap Editor when ydoc/provider refs change
- `readOnly` prop for View shares: the editor and title input are not editable. This is only UX; the editor service is what drops a read-only user's updates
- Title: an `<input>` bound to `titleText` from useCollaboration, written with `updateTitle()`
- `onTitleChange` calls `noteStore.updateNoteTitle()`, which only updates the sidebar. The editor service persists the title to the notes service after each save

---

## Why a composable instead of a Pinia store

1. **Lifecycle mismatch** — Pinia store is app-scoped singleton; collaboration state is per-editing-session
2. **Instance vs singleton** — Multiple editors possible (e.g. side-by-side)
3. **SSR safety** — Y.Doc and WebSocket cannot exist on the server
4. **CRDT objects aren't serializable** — Can't be part of `window.__INITIAL_STATE__`

---

## Extension configuration

- **`undoRedo: false`** — Disables StarterKit's undo/redo. Collaboration extension provides CRDT-aware undo that only reverts your changes.
- **`Collaboration.configure({ document: ydoc })`** — Tiptap internally creates `XmlFragment("default")`. Uses the full Y.Doc.
- **`CollaborationCaret.configure({ provider: prov })`** — Reads `provider.awareness` for shared Awareness instance.

---

## Title handling

The title uses `Y.Text` directly (not Tiptap):

```
User types → updateTitle() → yTitle.delete() + yTitle.insert()
  → Y.Doc update → WebSocket → other clients
  → yTitle.observe() → titleText.value updates → InputText rerenders
```

---

## SSR Safety

`NoteEditor` creates a WebSocket in setup, which crashes Node.js. Guard in `pages/Notes.vue`:

```typescript
const mounted = ref(false);
onMounted(() => { mounted.value = true; });
```
```vue
<NoteEditor v-if="mounted && activeNote" ... />
```

The guard must be one level up — even with `v-if` inside NoteEditor, the `<script setup>` would already have executed.

---

## Known Limitations

1. **No offline persistence** — Local edits survive reconnect (in-memory) but lost if tab closes while disconnected
