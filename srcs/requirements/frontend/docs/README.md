# Frontend Documentation

## Collaborative Editing (`dev/collab/`)

| File | Description |
|------|-------------|
| [overview.md](dev/collab/overview.md) | The whole session: connect → load → sync → edit → persist → disconnect, across nginx, the editor service and the notes service. |
| [tiptap-yjs-integration.md](dev/collab/tiptap-yjs-integration.md) | Frontend integration: useCollaboration composable, NoteEditor component, shared title, SSR safety. |
| [live-cursors.md](dev/collab/live-cursors.md) | Awareness protocol: ephemeral cursor/selection data, CollaborationCaret config, connected users count. |

## Sharing (`dev/`)

| File | Description |
|------|-------------|
| [share-api.md](dev/share-api.md) | Share endpoints (auth + notes services), response types and how the Notes page uses them. |

## Internationalization (`dev/`)

| File | Description |
|------|-------------|
| [i18n.md](dev/i18n.md) | i18n architecture: frontend useUiI18n composable + JSON dictionaries, backend Fluent FTL files, error localization, locale resolution, consuming backend errors in Vue, best practices. |

## Vue + Volt UI (`dev/vue/`)

General patterns written while learning the stack; the example component names are illustrative.

| File | Description |
|------|-------------|
| [vue-volt-patterns.md](dev/vue/vue-volt-patterns.md) | Vue 3.5 features (`useTemplateRef`, reactive destructuring, `useId`, lazy hydration), Volt UI setup, Pinia stores, SSR-safe patterns. |
| [composables.md](dev/vue/composables.md) | Vue 3 composable patterns: creating, using, SSR safety, composables vs Pinia. |

## Styling (`dev/`)

| File | Description |
|------|-------------|
| [tailwind-class-strategies.md](dev/tailwind-class-strategies.md) | 5 strategies for managing Tailwind classes: `@apply`, layout wrappers, `@layer components`, JS variables, custom components. |

## Devlog (`dev/devlog/`)

Snapshots from the early notes view (February 2026); the components they describe have since been replaced.

| Date | Summary |
|------|---------|
| [2026-02-01](dev/devlog/2026-02-01-notes-api-integration.md) | Notes API integration and creation |
| [2026-02-01](dev/devlog/2026-02-01-notes-view-master-detail.md) | NotesView master-detail layout with Volt UI |
| [2026-02-02](dev/devlog/2026-02-02-notes-view-cleanup.md) | NotesView cleanup and Listbox fix |
| [2026-02-02](dev/devlog/2026-02-02-notes-view-modularization.md) | NotesView modularization |
| [2026-02-02](dev/devlog/2026-02-02-sidebar-layout-navigation.md) | SidebarLayout and navigation |
