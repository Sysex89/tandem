# Frontend

Nuxt 4 (Vue 3) app with server-side rendering: the login and notes pages, the collaborative TipTap editor, sharing, account settings, 8 locales and light/dark themes. Served behind nginx at `/`.

## Running it

In the full stack (from the repository root):

```bash
make demo   # production image, seeded with alice / bob
make dev    # bind-mounts this folder, hot reload with `nuxt dev`
```

On its own (`pnpm install` first):

```bash
pnpm dev         # dev server on http://localhost:3000 (the API calls need the stack's nginx)
pnpm typecheck   # strict; CI requires 0 errors
pnpm build
```

## Layout

| Path | What |
| --- | --- |
| `app/pages/` | `Login`, `Notes` (sidebar, editor, sharing dialog), `Account`, `Home`, `Privacy`, `Terms` |
| `app/middleware/auth.global.ts` | Redirects to `/login` unless the route is public or the session is valid |
| `app/stores/` | Pinia: `authStore` (session, profile), `noteStore` (note list, selection) |
| `app/composables/useCollaboration.ts` | Y.Doc + y-websocket provider per note, shared title, connected-user count |
| `app/components/notes/NoteEditor.vue` | TipTap editor with the Collaboration and CollaborationCaret extensions |
| `app/utils/caretRenderer.ts` | Per-user caret color and name label |
| `app/composables/useUiI18n.ts`, `app/locales/` | i18n: flat dot-keyed JSON per locale; a new key goes in every file |
| `app/volt/` | Only the [Volt](https://volt.primevue.org/) components actually used; add more with the Volt CLI |

During SSR the stores call the API through nginx's internal listener (`http://nginx:8081`), so `auth_request` applies there too; in the browser they use relative `/api/...` URLs.

## Docs

Design notes and devlogs are in [`docs/`](docs/README.md): the collaboration pipeline, live cursors, i18n, the sharing API, and Vue/Volt/Tailwind patterns.
