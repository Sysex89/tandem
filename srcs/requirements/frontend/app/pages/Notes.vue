<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import Listbox from "@/volt/Listbox.vue";
import Button from "@/volt/Button.vue";
import Dialog from "@/volt/Dialog.vue";
import SelectButton from "@/volt/SelectButton.vue";
import SidebarLayout from "@/components/layouts/SidebarLayout.vue";
import NoteEditor from "@/components/notes/NoteEditor.vue";
import { useConfirm } from "primevue/useconfirm";
import TimesIcon from "@primevue/icons/times";
import { useNoteStore } from "@/stores/noteStore";
import { useUiI18n } from "~/composables/useUiI18n";
import { useAuthStore } from "@/stores/authStore";
import UserAvatar from "@/components/UserAvatar.vue";
import type { Note, SharedNote, Collaborator } from "@/types";

type ShareUser = { id: string; loginName: string; imageURL: string | null };

const noteStore = useNoteStore();
const confirm = useConfirm();
const { t } = useUiI18n();
const authStore = useAuthStore();

const isMobile = useMediaQuery('(max-width: 767px)');
const sidebarOpen = ref(true);

const noteEditorRef = ref<InstanceType<typeof NoteEditor> | null>(null);

const showInviteDialog = ref(false);
const inviteRole = ref<"Edit" | "View">("Edit");
const roleOptions = computed(() => [
	{ label: t("notes.invite.role.Edit"), value: "Edit" },
	{ label: t("notes.invite.role.View"), value: "View" },
]);
const inviteNoteId = ref<string | null>(null);
const selectedUsers = ref<string[]>([]);
const allUsers = ref<ShareUser[]>([]);
const collaborators = ref<Collaborator[]>([]);
const inviteError = ref('');
const inviteSuccess = ref('');

async function fetchUsers() {
	try {
		const res = await $fetch<{ users: ShareUser[] }>('/api/auth/users');
		allUsers.value = res.users;
	} catch {
		allUsers.value = [];
	}
}

function imageFor(userId: string | null): string | null {
	if (!userId) return null;
	return allUsers.value.find(u => u.id === userId)?.imageURL ?? null;
}

async function fetchCollaborators(noteId: string) {
	try {
		collaborators.value = await $fetch(`/api/notes/collab/${noteId}`);
	} catch {
		collaborators.value = [];
	}
}

function openInvite(noteId: string) {
	inviteNoteId.value = noteId;
	selectedUsers.value = [];
	inviteError.value = '';
	inviteSuccess.value = '';
	showInviteDialog.value = true;
	if (allUsers.value.length === 0) fetchUsers();
	fetchCollaborators(noteId);
}

function toggleUser(userId: string) {
	const idx = selectedUsers.value.indexOf(userId);
	if (idx === -1) {
		selectedUsers.value.push(userId);
	} else {
		selectedUsers.value.splice(idx, 1);
	}
}

function usernameFor(userId: string | null) {
	if (!userId) return t('notes.publicLink');
	return allUsers.value.find(u => u.id === userId)?.loginName ?? userId.slice(0, 8);
}

function displayTitle(title: string | null | undefined) {
	const value = (title ?? '').trim();
	if (!value) return t('notes.untitled');
	if (value.toLowerCase() === 'untitled') return t('notes.untitled');
	return value;
}

// Users not already collaborators and not the current user
const availableUsers = computed(() => {
	const collabIds = new Set(collaborators.value.map(c => c.guest_id));
	const currentUserId = authStore.user?.id;
	return allUsers.value.filter(u => !collabIds.has(u.id) && u.id !== currentUserId);
});

async function revokeShare(shareId: string) {
	inviteError.value = '';
	try {
		await $fetch(`/api/notes/collab/revoke/${shareId}`, { method: 'DELETE' });
		collaborators.value = collaborators.value.filter(c => c.share_id !== shareId);
		inviteSuccess.value = t('notes.invite.revoked');
	} catch {
		inviteError.value = t('notes.invite.error.revokeFailed');
	}
}

async function sendInvite() {
	if (selectedUsers.value.length === 0 || !inviteNoteId.value) return;
	inviteError.value = '';
	inviteSuccess.value = '';

	let shared = 0;
	let skipped = 0;
	for (const guestId of selectedUsers.value) {
		try {
			await $fetch('/api/notes/collab/', {
				method: 'POST',
				body: { note_id: inviteNoteId.value, guest_id: guestId, role: inviteRole.value },
			});
			shared++;
		} catch (e: any) {
			const status = e?.response?.status ?? e?.status ?? e?.statusCode;
			if (status === 409) {
				skipped++;
			} else {
				const errorDetail = e?.data?.message || e?.data?.error || e?.message || e?.toString() || 'Unknown error';
				inviteError.value = `${t('notes.invite.error.shareSomeFailed')} (${status}): ${errorDetail}`;
			}
		}
	}

	if (shared > 0) {
		inviteSuccess.value = shared === 1
			? t('notes.invite.sharedCountSingle', { count: shared })
			: t('notes.invite.sharedCountPlural', { count: shared });
	}
	if (skipped > 0) inviteSuccess.value += skipped === selectedUsers.value.length
		? ` ${t('notes.invite.allSelectedAlreadyHaveAccess')}`
		: ` ${t('notes.invite.someAlreadyHadAccess', { count: skipped })}`;

	selectedUsers.value = [];
	if (inviteNoteId.value) fetchCollaborators(inviteNoteId.value);
}

const sharedNotes = ref<SharedNote[]>([]);
const selectedSharedNote = ref<SharedNote | null>(null);

async function fetchSharedNotes() {
	try {
		sharedNotes.value = await $fetch('/api/notes/collab/received');
	} catch {
		sharedNotes.value = [];
	}
}

async function checkAndClean() {
	const note = noteStore.selectedNote;
	if (note && noteEditorRef.value?.isEmpty) {
		await noteStore.deleteNote(note.id);
	}
}

async function selectOwnNote(note: Note) {
	await checkAndClean();
	noteStore.selectedNote = note;
	selectedSharedNote.value = null;
	if (isMobile.value) sidebarOpen.value = false;
}

async function selectSharedNote(note: SharedNote) {
	await checkAndClean();
	selectedSharedNote.value = note;
	noteStore.selectedNote = null;
	if (isMobile.value) sidebarOpen.value = false;
}

const activeNote = computed(() => {
	if (noteStore.selectedNote) return noteStore.selectedNote;
	if (selectedSharedNote.value) {
		const shared = selectedSharedNote.value;
		return { id: shared.note_id, title: shared.note_title, readOnly: shared.access_role === "View" };
	}
	return null;
});

// SSR guard — NoteEditor creates WebSocket in setup, which crashes Node
const mounted = ref(false);

async function handleEsc(e: KeyboardEvent) {
	if (e.key !== 'Escape' || !activeNote.value) return;
	await checkAndClean();
	noteStore.selectedNote = null;
	selectedSharedNote.value = null;
}

// Refresh share state when the tab regains focus so a user who was
// granted / revoked access in another session sees it without reloading.
// The editor WS already handles live doc edits; this covers sidebar state.
let lastCollabRefresh = 0;
function refreshCollabState() {
	if (document.visibilityState === 'hidden') return;
	const now = Date.now();
	if (now - lastCollabRefresh < 1000) return;
	lastCollabRefresh = now;
	fetchSharedNotes();
	if (showInviteDialog.value && inviteNoteId.value) {
		fetchCollaborators(inviteNoteId.value);
	}
}

onMounted(() => {
	mounted.value = true;
	document.addEventListener('keydown', handleEsc);
	document.addEventListener('visibilitychange', refreshCollabState);
	window.addEventListener('focus', refreshCollabState);
	if (isMobile.value) sidebarOpen.value = false;
	fetchSharedNotes();
});
onBeforeUnmount(() => {
	document.removeEventListener('keydown', handleEsc);
	document.removeEventListener('visibilitychange', refreshCollabState);
	window.removeEventListener('focus', refreshCollabState);
	checkAndClean();
});

const isCreating = ref(false);
async function handleCreate() {
	if (isCreating.value) return;
	isCreating.value = true;
	await checkAndClean();
	await noteStore.createNote();
	await nextTick();
	noteEditorRef.value?.focusTitle();
	isCreating.value = false;
}

async function confirmDelete(id: string) {
	if (noteStore.selectedNote?.id === id && noteEditorRef.value?.isEmpty) {
		await noteStore.deleteNote(id);
		return;
	}

	confirm.require({
		message: t('notes.delete.confirmMessage'),
		header: t('notes.delete.confirmHeader'),
		icon: 'pi pi-trash',
		acceptProps: {
			label: t('notes.delete.confirmAccept'),
			severity: 'danger'
		},
		rejectProps: {
			label: t('notes.delete.confirmReject'),
			severity: 'secondary'
		},
		accept: async () => {
			await noteStore.deleteNote(id);
		}
	});
}

await useAsyncData('notes', async () => {
	if (noteStore.notesCount === 0) {
		await noteStore.fetchNotes();
	}
});
</script>

<template>
	<SidebarLayout v-model:sidebar-open="sidebarOpen">
		<template #sidebar>
			<!-- Toolbar row -->
			<div class="flex items-center gap-1 mb-2">
				<button
					class="w-7 h-7 shrink-0 flex items-center justify-center rounded-md text-surface-400 hover:text-surface-700 hover:bg-surface-200 dark:hover:text-surface-200 dark:hover:bg-surface-700 transition-colors"
					:title="sidebarOpen ? t('notes.sidebar.hide') : t('notes.sidebar.show')"
					@click="sidebarOpen = !sidebarOpen">
					<IconBars class="w-4 h-4" />
				</button>
			</div>

			<template v-if="sidebarOpen">
				<div class="flex items-center justify-between mb-1 px-2">
					<h2 class="section-title !mb-0">{{ t('notes.title') }}</h2>
					<Button label="+" text rounded @click="handleCreate" />
				</div>
				<p v-if="noteStore.error" class="error-text">{{ noteStore.error }}</p>
				<Listbox :model-value="noteStore.selectedNote" @update:model-value="selectOwnNote"
					:options="noteStore.notes" optionLabel="title" dataKey="id"
					pt:root:class="!border-0 !shadow-none !bg-transparent" pt:list:class="!p-0 !gap-0.5"
					pt:listContainer:class="!overflow-visible !max-h-none" pt:option:class="!px-2 !py-1.5 !rounded-md">
					<template #empty>
						<button
							class="px-2 py-1.5 text-sm text-primary-400 hover:text-primary-300 transition-colors w-full text-left"
							@click="handleCreate">
							{{ t('notes.createFirst') }}
						</button>
					</template>
					<template #option="slotProps">
						<div class="flex items-center justify-between w-full group/item gap-1">
							<span class="truncate text-sm">{{ displayTitle(slotProps.option.title) }}</span>
							<div class="flex items-center shrink-0"
								:class="noteStore.selectedNote?.id === slotProps.option.id ? '' : 'opacity-0 group-hover/item:opacity-100 transition-opacity'">
								<button class="w-6 h-6 rounded-full flex items-center justify-center transition-colors"
									:class="noteStore.selectedNote?.id === slotProps.option.id
										? 'text-white hover:bg-white/20'
										: 'text-surface-400 hover:text-surface-0 hover:bg-surface-600'"
									:aria-label="t('notes.invite.header')" @click.stop="openInvite(slotProps.option.id)">
									<IconUserPlus class="w-3.5 h-3.5" />
								</button>
								<button class="w-6 h-6 rounded-full flex items-center justify-center transition-colors"
									:class="noteStore.selectedNote?.id === slotProps.option.id
										? 'text-white hover:text-red-300 hover:bg-white/20'
										: 'text-surface-400 hover:text-red-400 hover:bg-surface-600'"
									@click.stop="confirmDelete(slotProps.option.id)">
									<TimesIcon class="w-2.5 h-2.5" />
								</button>
							</div>
						</div>
					</template>
				</Listbox>

				<div v-if="sharedNotes.length" class="flex items-center justify-between mt-4 mb-1 px-2">
					<h2 class="section-title !mb-0">{{ t('notes.shared') }}</h2>
				</div>
				<Listbox v-if="sharedNotes.length" :model-value="selectedSharedNote"
					@update:model-value="selectSharedNote" :options="sharedNotes" optionLabel="note_title"
					dataKey="note_id" pt:root:class="!border-0 !shadow-none !bg-transparent"
					pt:list:class="!p-0 !gap-0.5" pt:listContainer:class="!overflow-visible !max-h-none"
					pt:option:class="!px-2 !py-1.5 !rounded-md">
					<template #option="slotProps">
						<div class="flex items-center justify-between w-full">
							<span class="truncate text-sm">{{ slotProps.option.note_title || t('notes.untitled')
							}}</span>
							<span class="text-xs text-surface-500 shrink-0 ml-2">{{
								t(`notes.invite.role.${slotProps.option.access_role}`) }}</span>
						</div>
					</template>
				</Listbox>

			</template>

		</template>

		<div v-if="mounted && activeNote"
			class="flex flex-1 w-full h-full gap-4 flex-col md:flex-row overflow-y-auto md:overflow-y-visible">
			<NoteEditor ref="noteEditorRef" :note-id="activeNote.id" :fallback-title="activeNote.title"
				:read-only="'readOnly' in activeNote && activeNote.readOnly" class="flex-1" />
		</div>
		<div v-else-if="!activeNote" class="empty-state">{{ t('notes.empty') }}</div>

		<Dialog v-model:visible="showInviteDialog" :header="t('notes.invite.header')" modal :draggable="false"
			:dismissableMask="false" pt:root:class="w-full max-w-sm">
			<div class="flex flex-col gap-3">
				<!-- Current collaborators -->
				<div v-if="collaborators.length">
					<label class="text-sm text-surface-500 mb-1 block">{{ t('notes.invite.currentAccess') }}</label>
					<div class="flex flex-col rounded-md border border-surface-700 overflow-hidden">
						<div v-for="collab in collaborators" :key="collab.share_id"
							class="flex items-center justify-between px-3 py-2 text-sm text-surface-300">
							<div class="flex items-center gap-3 min-w-0">
								<UserAvatar v-if="collab.guest_id" :uuid="collab.guest_id"
									:image-u-r-l="imageFor(collab.guest_id)" :size="28"
									class="shrink-0 rounded-full overflow-hidden" />
								<div class="min-w-0">
									<div class="truncate">@{{ usernameFor(collab.guest_id) }}</div>
									<div class="text-xs text-surface-500">{{ t(`notes.invite.role.${collab.access_role}`) }}</div>
								</div>
							</div>
							<button
								class="w-6 h-6 rounded-full flex items-center justify-center text-surface-400 hover:text-red-400 hover:bg-surface-600 transition-colors shrink-0"
								@click="revokeShare(collab.share_id)">
								<TimesIcon class="w-2.5 h-2.5" />
							</button>
						</div>
					</div>
				</div>

				<!-- Selected users to invite -->
				<div v-if="selectedUsers.length" class="flex flex-wrap gap-1.5">
					<span v-for="uid in selectedUsers" :key="uid"
						class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs bg-primary-500/15 text-primary-400">
						{{allUsers.find(u => u.id === uid)?.loginName ?? uid}}
						<button class="hover:text-primary-300" @click="toggleUser(uid)">
							<TimesIcon class="w-2 h-2" />
						</button>
					</span>
				</div>

				<!-- Available users to add -->
				<label class="text-sm text-surface-500">{{ t('notes.invite.selectUsers') }}</label>
				<div v-if="availableUsers.length"
					class="flex flex-col rounded-md border border-surface-700 overflow-hidden">
					<button v-for="user in availableUsers" :key="user.id"
						class="flex items-center gap-3 px-3 py-2 text-sm text-left transition-colors" :class="selectedUsers.includes(user.id)
							? 'bg-primary-500/15 text-primary-400'
							: 'hover:bg-surface-800 text-surface-300'" @click="toggleUser(user.id)">
						<UserAvatar :uuid="user.id" :image-u-r-l="user.imageURL" :size="28"
							class="shrink-0 rounded-full overflow-hidden" />
						<div class="min-w-0">
							<div class="truncate">@{{ user.loginName }}</div>
						</div>
					</button>
				</div>
				<div v-else class="text-xs text-surface-500">{{ t('notes.invite.allUsersHaveAccess') }}</div>

				<div v-if="availableUsers.length" class="flex items-center justify-between gap-2">
					<label class="text-sm text-surface-500">{{ t('notes.invite.permission') }}</label>
					<SelectButton v-model="inviteRole" :options="roleOptions" optionLabel="label" optionValue="value"
						:allowEmpty="false" />
				</div>

				<span v-if="inviteError" class="text-xs text-red-500">{{ inviteError }}</span>
				<span v-if="inviteSuccess" class="text-xs text-green-600">{{ inviteSuccess }}</span>
			</div>
			<template #footer>
				<Button :label="t('notes.invite.send')" :disabled="selectedUsers.length === 0" @click="sendInvite" />
			</template>
		</Dialog>
	</SidebarLayout>
</template>