<script setup lang="ts">
import { shallowRef, ref, computed, watchEffect } from "vue";
import { Editor, EditorContent } from "@tiptap/vue-3";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import Collaboration from "@tiptap/extension-collaboration";
import CollaborationCaret from "@tiptap/extension-collaboration-caret";
import { useCollaboration } from "@/composables/useCollaboration";
import { useNoteStore } from "@/stores/noteStore";
import { useAuthStore } from "@/stores/authStore";
import { userColor, createCaretRenderer } from "@/utils/caretRenderer";
import { useUiI18n } from "~/composables/useUiI18n";

const props = defineProps<{
  noteId: string;
  // Title stored in the notes service; shown until the shared title has been set.
  fallbackTitle?: string;
  // View-only share: the editor service drops this user's edits, so don't offer editing.
  readOnly?: boolean;
}>();

const noteStore = useNoteStore();
const authStore = useAuthStore();
const { t } = useUiI18n();
const { ydoc, provider, titleText, connectedUsers, updateTitle } =
  useCollaboration(() => props.noteId, (id, title) => noteStore.updateNoteTitle(id, title));

const editor = shallowRef<Editor>();

watchEffect((onCleanup) => {
  const doc = ydoc.value;
  const prov = provider.value;
  if (!doc || !prov) return;

  const renderCaret = createCaretRenderer(prov.awareness) as (user: Record<string, any>) => HTMLElement;

  const ed = new Editor({
    editable: !props.readOnly,
    extensions: [
      StarterKit.configure({ undoRedo: false }),
      Placeholder.configure({ placeholder: t('noteEditor.placeholder') }),
      Collaboration.configure({ document: doc }),
      CollaborationCaret.configure({
        provider: prov,
        user: {
          name: authStore.user?.loginName ?? t('noteEditor.anonymous'),
          color: userColor(authStore.user?.id),
        },
        render: renderCaret,
      }),
    ],
  });

  editor.value = ed;
  onCleanup(() => ed.destroy());
});

// make sure the editor is connected
const isEmpty = computed(() =>
  !!ydoc.value && !titleText.value.trim() && (!editor.value || editor.value.isEmpty)
);

const titleInput = ref<HTMLInputElement | null>(null);

function focusTitle() {
  titleInput.value?.focus();
}

defineExpose({ isEmpty, focusTitle });

function focusEditorEnd(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (target.closest(".editor-scroll")) return;
  const ed = editor.value;
  if (!ed) return;
  ed.commands.focus("end");
}
</script>

<template>
  <div ref="wrapper" class="editor-wrapper" @click="focusEditorEnd">
  <div class="editor-surface">
    <div class="editor-scroll">
      <input
        ref="titleInput"
        :value="titleText"
        :readonly="readOnly"
        @input="updateTitle(($event.target as HTMLInputElement).value)"
        @keydown.enter.prevent="editor?.commands.focus('start')"
        :placeholder="fallbackTitle || t('notes.untitled')"
        class="editor-title"
      />
      <EditorContent :editor="editor" class="tiptap-editor" />
    </div>
    <div v-if="readOnly || connectedUsers > 1" class="editor-status">
      <span v-if="readOnly" class="editor-status-readonly">{{ t('noteEditor.viewOnly') }}</span>
      <template v-if="connectedUsers > 1">
        <span class="editor-status-dot" />
        {{ connectedUsers }} {{ t('noteEditor.collaborators') }}
      </template>
    </div>
  </div>
  </div>
</template>
