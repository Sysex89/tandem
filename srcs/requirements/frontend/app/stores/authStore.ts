import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { useUiI18n } from "~/composables/useUiI18n";

interface User {
	id: string;
	email: string | null;
	role: string | null;
	loginName: string | null;
	imageURL: string | null;
	hasLocalAuth: boolean;
}

export const useAuthStore = defineStore("auth", () => {
	const { t } = useUiI18n();
	const user = ref<User | null>(null);
	const loading = ref(false);
	const error = ref<string | null>(null);
	const sessionCookie = ref<string | null>(null); // Store cookie for SSR requests

	const isAuthenticated = computed(() => !!user.value);

	// Check if session cookie is valid
	async function checkAuth(serverCookie?: string, force = false) {
		// Capture headers at the very start (Magic must happen before any await)
		const isServer = typeof window === "undefined";
		const capturedCookie = serverCookie || (isServer ? useRequestHeaders(["cookie"]).cookie : undefined);

		// If user is already set, skip unless forced refresh is requested.
		if (user.value && !force) return;

		loading.value = true;
		error.value = null;
		if (capturedCookie) sessionCookie.value = capturedCookie;

		try {
			// Headers setup
			const headers: HeadersInit = {};
			if (isServer && capturedCookie) {
				headers["Cookie"] = capturedCookie;
			}

			// Thick Check: Fetch Profile (This internally verifies the session)
			// If server, we MUST use the internal docker network URL
			// If client, we use the relative URL (proxied by Nginx)
			const url = isServer ? "http://nginx:8081/api/auth/me" : "/api/auth/me";

			const res = await fetch(url, { headers });
			if (res.ok) {
				const data = await res.json();
				user.value = data.user;
			} else {
				user.value = null; // Session invalid or profile not found
			}
		} catch (e) {
			console.error("Auth check failed", e);
			user.value = null;
		} finally {
			loading.value = false;
		}
	}

	function resetStore() {
		user.value = null;
		error.value = null;
		sessionCookie.value = null;
	}

	async function logout() {
		loading.value = true;
		try {
			await fetch("/api/auth/logout", { method: "POST" });
		} catch (e) {
			console.error("Logout failed", e);
		} finally {
			resetStore();
			// Clear Note Store
			// Dynamic import to avoid circular dependency since noteStore uses authStore
			try {
				const { useNoteStore } = await import("@/stores/noteStore");
				const noteStore = useNoteStore();
				noteStore.resetStore();
			} catch (err) {
				console.error("Failed to reset note store", err);
			}

			loading.value = false;
			window.location.href = "/";
		}
	}

	// Helper for manual login (if we build a form)
	async function loginLocal(identifier: string, password: string) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/login", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ identifier, password }),
			});

			if (!res.ok) {
				const data = await res.json();
				throw new Error(data.message || data.error || t("auth.error.loginFailed"));
			}

			// success
			await checkAuth(); // refresh user data
			return true;
		} catch (e: any) {
			error.value = e.message;
			return false;
		} finally {
			loading.value = false;
		}
	}

	async function registerLocal(loginName: string, email: string, password: string) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/register", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ loginName, email, password }),
			});

			if (!res.ok) {
				const data = await res.json();
				throw new Error(data.message || data.error || t("auth.error.registrationFailed"));
			}

			// success
			await checkAuth();
			return true;
		} catch (e: any) {
			error.value = e.message;
			return false;
		} finally {
			loading.value = false;
		}
	}

	async function updateLoginName(newLogin: string) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/change-login", {
				method: "PATCH",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ loginName: newLogin }),
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.updateFailed"));

			await checkAuth(undefined, true); // Refresh profile
			return { success: true, message: data.message };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	async function updateEmail(newEmail: string) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/change-email", {
				method: "PATCH",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ email: newEmail }),
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.updateFailed"));

			await checkAuth(undefined, true); // Refresh profile
			return { success: true, message: data.message };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	async function updateImageUrl(imageURL: string | null) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/change-image", {
				method: "PATCH",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ imageURL }),
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.imageUpdateFailed"));
			await checkAuth(undefined, true);
			return { success: true, message: data.message };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	async function changePassword(oldPassword: string, newPassword: string) {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/change-password", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ oldPassword, newPassword }),
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.passwordChangeFailed"));
			return { success: true, message: data.message };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	async function deleteAccount() {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/delete-account", { method: "DELETE" });
			const data = await res.json().catch(() => ({}));
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.accountDeletionFailed"));

			resetStore();
			try {
				const { useNoteStore } = await import("@/stores/noteStore");
				const noteStore = useNoteStore();
				noteStore.resetStore();
			} catch (err) {
				console.error("Failed to reset note store", err);
			}

			return { success: true, message: data.message || t("auth.success.accountDeleted") };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	async function exportData() {
		loading.value = true;
		error.value = null;
		try {
			const res = await fetch("/api/auth/export-data", { method: "GET" });
			const data = await res.json().catch(() => ({}));
			if (!res.ok) throw new Error(data.message || data.error || t("auth.error.dataExportFailed"));
			return { success: true, data };
		} catch (e: any) {
			error.value = e.message;
			return { success: false, message: e.message };
		} finally {
			loading.value = false;
		}
	}

	return {
		user,
		isAuthenticated,
		loading,
		error,
		checkAuth,
		sessionCookie,
		logout,
		resetStore,
		loginLocal,
		registerLocal,
		updateLoginName,
		updateEmail,
		updateImageUrl,
		changePassword,
		deleteAccount,
		exportData,
	};
});
