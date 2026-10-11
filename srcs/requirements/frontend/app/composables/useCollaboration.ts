import { ref, shallowRef, watch, onUnmounted, type Ref, type ShallowRef } from "vue";
import * as Y from "yjs";
import { WebsocketProvider } from "y-websocket";

interface UseCollaborationReturn {
  ydoc: ShallowRef<Y.Doc | null>;
  provider: ShallowRef<WebsocketProvider | null>;
  titleText: Ref<string>;
  connectedUsers: Ref<number>;
  updateTitle: (value: string) => void;
}

// onTitleChange fires only when the shared title is edited (locally or by a
// collaborator), never when switching notes, so callers can persist it safely.
export function useCollaboration(
  noteId: () => string,
  onTitleChange?: (id: string, title: string) => void,
): UseCollaborationReturn {
  const currentYdoc = shallowRef<Y.Doc | null>(null);
  const currentProvider = shallowRef<WebsocketProvider | null>(null);
  const titleText = ref("");
  const connectedUsers = ref(0);

  // Pending connection that hasn't synced yet
  let pendingYdoc: Y.Doc | null = null;
  let pendingProvider: WebsocketProvider | null = null;

  function setup(id: string) {
    // Cancel any pending (not yet synced) setup
    pendingProvider?.destroy();
    pendingYdoc?.destroy();

    const newYdoc = new Y.Doc();
    const yTitle = newYdoc.getText("title");

    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${location.host}/ws`;
    const newProvider = new WebsocketProvider(wsUrl, id, newYdoc);

    pendingYdoc = newYdoc;
    pendingProvider = newProvider;

    // Wait for first sync before swapping — keeps old editor visible
    newProvider.on("sync", (isSynced: boolean) => {
      if (!isSynced) return;

      // If another setup started since, this one was cancelled
      if (pendingYdoc !== newYdoc) return;

      // Tear down old
      currentProvider.value?.destroy();
      currentYdoc.value?.destroy();

      // Activate new
      titleText.value = yTitle.toString();
      connectedUsers.value = newProvider.awareness.getStates().size;
      yTitle.observe(() => {
        titleText.value = yTitle.toString();
        onTitleChange?.(id, titleText.value);
      });
      newProvider.awareness.on("change", () => { connectedUsers.value = newProvider.awareness.getStates().size; });

      currentProvider.value = newProvider;
      currentYdoc.value = newYdoc;
      pendingYdoc = null;
      pendingProvider = null;
    });
  }

  watch(noteId, (id) => setup(id), { immediate: true });

  onUnmounted(() => {
    pendingProvider?.destroy();
    pendingYdoc?.destroy();
    currentProvider.value?.destroy();
    currentYdoc.value?.destroy();
  });

  function updateTitle(value: string): void {
    const ydoc = currentYdoc.value;
    if (!ydoc) return;
    const yTitle = ydoc.getText("title");
    ydoc.transact(() => {
      yTitle.delete(0, yTitle.length);
      yTitle.insert(0, value);
    });
  }

  return {
    ydoc: currentYdoc,
    provider: currentProvider,
    titleText,
    connectedUsers,
    updateTitle,
  };
}
