<script setup lang="ts">
import { ref, onMounted, reactive, computed } from 'vue';
import { useConfirm } from 'primevue/useconfirm';
import { useAuthStore } from '../stores/authStore';
import Card from '../volt/Card.vue';
import Divider from '../volt/Divider.vue';
import InputText from '../volt/InputText.vue';
import Password from '../volt/Password.vue';
import Button from '../volt/Button.vue';
import { useUiI18n } from '~/composables/useUiI18n';

const auth = useAuthStore();
const confirm = useConfirm();
const { t } = useUiI18n();
const activeAction = ref<'login' | 'email' | 'image' | 'removeImage' | 'password' | 'delete' | 'export' | 'logout' | null>(null);
const successMessage = ref('');

async function handleLogout() {
	activeAction.value = 'logout';
	await auth.logout();
	activeAction.value = null;
}

const formLogin = ref('');
const formEmail = ref('');
const oldPassword = ref('');
const newPassword = ref('');
const confirmPassword = ref('');

const formImageUrl = ref('');

const errors = reactive({
	login: '',
	email: '',
	password: '',
	image: ''
});

onMounted(() => {
	if (auth.user) {
		formLogin.value = '';
		formEmail.value = '';
	}
});

async function handleUpdateLogin() {
	if (!formLogin.value || formLogin.value === auth.user?.loginName) return;

	activeAction.value = 'login';
	errors.login = '';
	successMessage.value = '';

	const result = await auth.updateLoginName(formLogin.value);

	if (!result.success) {
		errors.login = result.message;
	} else {
		formLogin.value = '';
		successMessage.value = t('profile.success.username');
	}

	activeAction.value = null;
}

async function handleUpdateEmail() {
	if (formEmail.value === auth.user?.email) return;

	activeAction.value = 'email';
	errors.email = '';
	successMessage.value = '';

	const result = await auth.updateEmail(formEmail.value);

	if (!result.success) {
		errors.email = result.message;
	} else {
		successMessage.value = t('profile.success.email');
	}

	activeAction.value = null;
}

async function handleUpdateImage() {
	activeAction.value = 'image';
	errors.image = '';
	successMessage.value = '';

	const result = await auth.updateImageUrl(formImageUrl.value || null);

	if (!result.success) {
		errors.image = result.message;
	} else {
		formImageUrl.value = '';
		successMessage.value = t('profile.success.image');
	}

	activeAction.value = null;
}

async function handleRemoveImage() {
	activeAction.value = 'removeImage';
	errors.image = '';
	successMessage.value = '';

	const result = await auth.updateImageUrl(null);

	if (!result.success) {
		errors.image = result.message;
	} else {
		formImageUrl.value = '';
		successMessage.value = t('profile.success.image');
	}

	activeAction.value = null;
}

async function handleChangePassword() {
	if (!newPassword.value) return;

	if (auth.user?.hasLocalAuth && !oldPassword.value) {
		errors.password = t('profile.error.oldPasswordRequired');
		return;
	}

	if (newPassword.value !== confirmPassword.value) {
		errors.password = t('profile.error.passwordMismatch');
		return;
	}

	activeAction.value = 'password';
	errors.password = '';
	successMessage.value = '';

	const result = await auth.changePassword(oldPassword.value, newPassword.value);

	if (!result.success) {
		errors.password = result.message;
	} else {
		oldPassword.value = '';
		newPassword.value = '';
		confirmPassword.value = '';
		successMessage.value = t('profile.success.password');
	}

	activeAction.value = null;
}

function handleDeleteAccount() {
	confirm.require({
		message: t('profile.delete.confirm'),
		header: t('profile.section.danger'),
		icon: 'pi pi-exclamation-triangle',
		acceptProps: {
			label: t('profile.button.deleteAccount'),
			severity: 'danger'
		},
		rejectProps: {
			label: t('notes.delete.confirmReject'),
			severity: 'secondary'
		},
		accept: async () => {
			activeAction.value = 'delete';
			successMessage.value = '';
			errors.login = '';
			errors.email = '';
			errors.password = '';

			const result = await auth.deleteAccount();
			if (!result.success) {
				errors.password = result.message;
				activeAction.value = null;
				return;
			}

			window.location.href = '/';
		}
	});
}

function extractPlainTextFromXml(xml: string): string {
	if (!xml) return '';
	const withBreaks = xml.replace(/<\/?(p|div|h[1-6]|li|br|tr|td)[^>]*>/gi, '\n');
	const stripped = withBreaks.replace(/<[^>]+>/g, '');
	return stripped.replace(/\n{2,}/g, '\n\n').trim();
}

async function decodeStateVector(stateVector: unknown): Promise<{ title: string; content: string } | null> {
	if (!Array.isArray(stateVector)) return null;
	const bytes = Uint8Array.from(stateVector.filter((v) => Number.isInteger(v) && v >= 0 && v <= 255));
	if (bytes.length === 0) return null;

	try {
		const Y = await import('yjs');
		const doc = new Y.Doc();
		Y.applyUpdate(doc, bytes);
		const title = doc.getText('title').toString();
		const contentXml = doc.getXmlFragment('default').toString();
		const content = extractPlainTextFromXml(contentXml);
		return { title, content };
	} catch {
		return null;
	}
}

async function buildReadableExportPayload(rawData: Record<string, unknown>) {
	if (!rawData || typeof rawData !== 'object') return rawData;
	if (!Array.isArray(rawData.notes)) return rawData;

	const notes = await Promise.all(rawData.notes.map(async (note: Record<string, unknown>) => {
		const decoded = await decodeStateVector(note?.state_vector);
		const { state_vector, ...rest } = note || {};
		return {
			...rest,
			title: decoded?.title || rest.title || t('notes.untitled'),
			content: decoded?.content || '',
		};
	}));

	return { ...rawData, notes };
}

async function handleExportData() {
	activeAction.value = 'export';
	successMessage.value = '';
	errors.login = '';
	errors.email = '';
	errors.password = '';

	const result = await auth.exportData();
	if (!result.success) {
		errors.password = result.message || t('auth.error.dataExportFailed');
		activeAction.value = null;
		return;
	}

	const payloadData = await buildReadableExportPayload(result.data as Record<string, unknown>);
	const payload = JSON.stringify(payloadData, null, 2);
	const blob = new Blob([payload], { type: 'application/json' });
	const url = URL.createObjectURL(blob);
	const now = new Date().toISOString().replace(/[:.]/g, '-');
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = `tandem-export-${now}.json`;
	document.body.appendChild(anchor);
	anchor.click();
	document.body.removeChild(anchor);
	URL.revokeObjectURL(url);

	successMessage.value = t('profile.export.success');
	activeAction.value = null;
}
</script>


<template>
	<div class="h-full overflow-y-auto p-4 bg-surface-50 dark:bg-surface-950">
		<div class="flex justify-center py-4">
		<Card class="w-full max-w-sm">
			<template #title>
				<h2 class="text-xl font-bold text-center">{{ t('profile.title') }}</h2>
				<p v-if="successMessage" class="text-sm text-green-500 font-normal mt-2 text-center text-wrap">{{
					successMessage }}</p>
			</template>
			<template #content>
				<div class="flex flex-col gap-6">

					<div class="flex flex-col gap-2">
						<h3 class="font-bold">{{ t('login.username') }}</h3>
						<p v-if="auth.user?.loginName">{{ t('profile.currentPrefix') }} <strong>{{ auth.user.loginName }}</strong></p>
						<form @submit.prevent="handleUpdateLogin" class="flex flex-col gap-2">
							<InputText v-model="formLogin" :placeholder="t('profile.placeholder.newUsername')" fluid />
							<Button :label="t('profile.button.updateUsername')" type="submit"
								:disabled="activeAction === 'login' || !formLogin || formLogin === (auth.user?.loginName || '')"
								fluid />
							<small v-if="errors.login" class="text-red-500">{{ errors.login }}</small>
						</form>
					</div>

					<div class="flex flex-col gap-2">
						<h3 class="font-bold">{{ t('login.email') }}</h3>
						<p>{{ t('profile.currentPrefix') }} <strong>{{ auth.user?.email || t('profile.none') }}</strong></p>
						<form @submit.prevent="handleUpdateEmail" class="flex flex-col gap-2">
							<InputText v-model="formEmail" :placeholder="t('profile.placeholder.newEmail')" fluid />
							<Button :label="t('profile.button.updateEmail')" type="submit"
								:disabled="activeAction === 'email' || formEmail === (auth.user?.email || '')" fluid />
							<small v-if="errors.email" class="text-red-500">{{ errors.email }}</small>
						</form>
					</div>

					<div class="flex flex-col gap-2">
						<h3 class="font-bold">{{ t('profile.section.avatar') }}</h3>
						<div v-if="auth.user?.imageURL" class="flex items-center gap-3">
							<img :src="auth.user.imageURL" :alt="t('profile.avatar.alt')" class="w-12 h-12 rounded-full object-cover" />
							<span class="text-sm text-muted-color truncate max-w-[160px]">{{ auth.user.imageURL }}</span>
						</div>
						<div class="flex flex-col gap-2">
							<InputText v-model="formImageUrl" :placeholder="t('profile.placeholder.imageUrl')" fluid />
							<Button :label="t('profile.button.updateImage')" :disabled="activeAction === 'image' || !formImageUrl"
								fluid @click="handleUpdateImage" />
							<Button v-if="auth.user?.imageURL" :label="t('profile.button.removeImage')"
								severity="secondary" :disabled="activeAction === 'removeImage'" fluid @click="handleRemoveImage" />
							<small v-if="errors.image" class="text-red-500">{{ errors.image }}</small>
						</div>
					</div>

					<div class="flex flex-col gap-2">
						<h3 class="font-bold">{{ t('profile.section.security') }}</h3>
						<form @submit.prevent="handleChangePassword" class="flex flex-col gap-2">
							<Password v-model="oldPassword" :placeholder="t('profile.placeholder.currentPassword')" :feedback="false" toggleMask
								fluid :disabled="!auth.user?.hasLocalAuth" />
							<Password v-model="newPassword" :placeholder="t('profile.placeholder.newPassword')" toggleMask fluid />
							<Password v-model="confirmPassword" :placeholder="t('profile.placeholder.confirmNewPassword')" :feedback="false"
								toggleMask fluid />
							<Button
								:label="auth.user?.hasLocalAuth ? t('profile.button.changePassword') : t('profile.button.addPassword')"
								type="submit"
								:disabled="activeAction === 'password' || !newPassword || !confirmPassword || newPassword !== confirmPassword"
								fluid />
							<small v-if="errors.password" class="text-red-500">{{ errors.password }}</small>
						</form>
					</div>
				<div class="flex flex-col gap-2 pt-2">
					<Button :label="t('profile.button.exportData')" severity="secondary" fluid :disabled="activeAction === 'export'"
						@click="handleExportData" />
					<Button :label="t('auth.logout')" severity="secondary" fluid :disabled="activeAction === 'logout'"
						@click="handleLogout" />
				</div>

				<div class="flex flex-col gap-2 pt-4">
					<h3 class="font-bold text-red-500">{{ t('profile.section.danger') }}</h3>
					<Button :label="t('profile.button.deleteAccount')" severity="danger" fluid :disabled="activeAction === 'delete'"
						@click="handleDeleteAccount" />
				</div>
				</div>
			</template>
		</Card>
		</div>
	</div>
</template>
