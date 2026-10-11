import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { useDebounceFn } from "@vueuse/core";
import type { Note } from "@/types";
import { useAuthStore } from "@/stores/authStore";
import { useUiI18n } from "~/composables/useUiI18n";

export const useNoteStore = defineStore("notes", () => {
	const authStore = useAuthStore();
	const { t } = useUiI18n();
	const notes = ref<Note[]>([]);
	const selectedNote = ref<Note | null>(null);
	const error = ref<string | null>(null);

	// Cache to prevent unnecessary refetches
	const CACHE_DURATION = 2 * 60 * 1000; // 2 minutes
	const lastFetchTimestamp = ref(0);
	const language = useCookie<string>("lang", { default: () => "en-UK" });

	function buildHeaders(extra: HeadersInit = {}): HeadersInit {
		const headers = new Headers(extra);
		headers.set("Accept-Language", language.value || "en-UK");
		return headers;
	}

	async function throwForResponse(response: Response): Promise<void> {
		if (response.ok) return;

		let message = `HTTP ${response.status}`;
		try {
			const data = (await response.json()) as { message?: string; error?: string };
			message = data?.message || data?.error || message;
		} catch {
			// keep fallback
		}
		throw new Error(message);
	}

	async function fetchNotes() {
		// Prevent redundant fetches
		const currentTime = Date.now();
		if (notes.value.length && currentTime - lastFetchTimestamp.value < CACHE_DURATION) {
			return;
		}

		error.value = null;
		let response: Response | null = null;

		try {
			// Ensure we have user profile before fetching notes.
			// The Auth store handles internal magic to capture the cookie
			if (!authStore.user) {
				await authStore.checkAuth();
				if (!authStore.user) return;
			}

			// On the server, we MUST use the full internal Docker URL.
			const isServer = typeof window === "undefined";
			const url = isServer ? "http://nginx:8081/api/notes" : "/api/notes";

			const userCookie = authStore.sessionCookie;
			const base = new Headers();
			if (isServer && userCookie) {
				base.set("Cookie", userCookie);
			}

			response = await fetch(url, {
				method: "GET",
				headers: buildHeaders(base),
				signal: AbortSignal.timeout(5000), // Prevent hanging
			});

			await throwForResponse(response);

			const fetchedNotes = await response.json();
			notes.value = fetchedNotes;

			lastFetchTimestamp.value = currentTime;
		} catch (catchError) {
			const errorMsg = catchError instanceof Error
				? (catchError.name === "AbortError" ? t("notes.error.requestTimedOut") : catchError.message)
				: t("notes.error.loadFailed");

			error.value = errorMsg;
			console.error("Failed to fetch notes:", errorMsg);
		}
	}

	async function createNote() {
		error.value = null;
		try {
			const response = await fetch("/api/notes", {
				method: "POST",
				headers: buildHeaders({ "Content-Type": "application/json" }),
				body: JSON.stringify({ title: t("notes.untitled") }),
			});

			await throwForResponse(response);

			const note: Note = await response.json();
			notes.value.push(note);
			selectedNote.value = note;
		} catch (catchError) {
			const errorMsg = catchError instanceof Error ? catchError.message : t("notes.error.createFailed");
			error.value = errorMsg;
			console.error("Failed to create note:", errorMsg);
		}
	}

	async function deleteNote(id: string) {
		error.value = null;
		try {
			const response = await fetch(`/api/notes/${id}`, {
				method: "DELETE",
			});

			if (!response.ok) throw new Error(`HTTP ${response.status}`);

			notes.value = notes.value.filter((n: Note) => n.id !== id);
			if (selectedNote.value?.id === id) {
				selectedNote.value = null;
			}
		} catch (catchError) {
			const errorMsg = catchError instanceof Error ? catchError.message : t("notes.error.deleteFailed");
			error.value = errorMsg;
			console.error("Failed to delete note:", errorMsg);
		}
	}

	const notesCount = computed(() => notes.value.length);

	// Debounced save to Postgres — temporary until WS notification channel
	const _persistTitle = useDebounceFn(async (id: string, title: string) => {
		try {
			const response = await fetch(`/api/notes/${id}`, {
				method: "PUT",
				headers: buildHeaders({ "Content-Type": "application/json" }),
				body: JSON.stringify({ title }),
			});
			await throwForResponse(response);
		} catch (e) {
			console.error("Failed to persist title:", e);
		}
	}, 500);

	function updateNoteTitle(id: string, title: string) {
		const note = notes.value.find((n: Note) => n.id === id);
		if (note) {
			note.title = title;
		}
		if (selectedNote.value?.id === id) {
			selectedNote.value = { ...selectedNote.value, title };
		}
		_persistTitle(id, title);
	}

	function resetSelected() {
		selectedNote.value = null;
	}

	function resetStore() {
		notes.value = [];
		selectedNote.value = null;
		error.value = null;
		lastFetchTimestamp.value = 0;
	}

	return {
		notes,
		selectedNote,
		error,
		notesCount,
		fetchNotes,
		createNote,
		deleteNote,
		resetSelected,
		updateNoteTitle,
		resetStore,
	};
});
