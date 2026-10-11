<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import Card from "@/volt/Card.vue";
import Button from "@/volt/Button.vue";
import InputText from "@/volt/InputText.vue";
import Checkbox from "@/volt/Checkbox.vue";
import Tabs from "@/volt/Tabs.vue";
import TabList from "@/volt/TabList.vue";
import Tab from "@/volt/Tab.vue";
import TabPanels from "@/volt/TabPanels.vue";
import TabPanel from "@/volt/TabPanel.vue";
import { useAuthStore } from "@/stores/authStore";
import { useUiI18n } from "~/composables/useUiI18n";

const authStore = useAuthStore();
const router = useRouter();
const { t } = useUiI18n();

const identifier = ref("");
const password = ref("");
const regLogin = ref("");
const regEmail = ref("");
const regPassword = ref("");
const consentAccepted = ref(false);

async function handleLogin() {
	authStore.error = null;
	if (!identifier.value.trim()) {
		authStore.error = t("login.validation.identifierRequired");
		return;
	}
	if (password.value.length < 8) {
		authStore.error = t("login.validation.passwordMin");
		return;
	}
	const success = await authStore.loginLocal(identifier.value, password.value);
	if (success) {
		router.push("/notes");
	}
}

async function handleRegister() {
	authStore.error = null;
	if (!consentAccepted.value) {
		authStore.error = t("login.consent.required");
		return;
	}
	if (regLogin.value.trim().length < 3) {
		authStore.error = t("login.validation.usernameRequired");
		return;
	}
	if (!regEmail.value.trim() || !regEmail.value.includes("@")) {
		authStore.error = t("login.validation.emailInvalid");
		return;
	}
	if (regPassword.value.length < 8) {
		authStore.error = t("login.validation.passwordMin");
		return;
	}
	const success = await authStore.registerLocal(regLogin.value, regEmail.value, regPassword.value);
	if (success) {
		router.push("/");
	}
}

</script>

<template>
	<div class="flex items-center justify-center h-full bg-gray-100 dark:bg-surface-950  p-4">
		<Card class="w-full max-w-md">
			<template #title>
				<div class="text-center text-2xl font-bold mb-4">{{ t('login.welcome') }}</div>
			</template>
			<template #content>
				<Tabs value="0">
					<TabList>
						<Tab value="0">{{ t('login.tab.login') }}</Tab>
						<Tab value="1">{{ t('login.tab.register') }}</Tab>
					</TabList>
					<TabPanels>
						<TabPanel value="0">
							<form class="flex flex-col gap-4 mt-4" @submit.prevent="handleLogin">
								<InputText v-model="identifier" :placeholder="t('login.identifier')" fluid />
								<InputText v-model="password" type="password" :placeholder="t('login.password')" fluid />
								<Button type="submit" :label="t('login.signin')" fluid />

								<div class="text-center text-sm text-gray-500 my-2">{{ t('login.or') }}</div>

								<a href="/api/auth/login/github" class="w-full">
									<Button :label="t('login.github')" icon="pi pi-github" severity="secondary" fluid />
								</a>
							</form>
						</TabPanel>
						<TabPanel value="1">
							<form class="flex flex-col gap-4 mt-4" @submit.prevent="handleRegister">
								<InputText v-model="regLogin" :placeholder="t('login.username')" fluid />
								<InputText v-model="regEmail" :placeholder="t('login.email')" fluid />
								<InputText v-model="regPassword" type="password" :placeholder="t('login.password')" fluid />

								<p class="text-xs text-gray-500 dark:text-gray-400">
									{{ t('login.consent.info') }}
								</p>
								<div class="flex items-start gap-2">
									<Checkbox v-model="consentAccepted" :binary="true" input-id="consent" />
									<label for="consent" class="text-sm text-gray-700 dark:text-gray-300 cursor-pointer select-none">
										{{ t('login.consent.label') }}
									</label>
								</div>

								<Button type="submit" :label="t('login.create')" :disabled="!consentAccepted" fluid />
							</form>
						</TabPanel>
					</TabPanels>
				</Tabs>

				<div v-if="authStore.error" class="text-red-500 text-sm mt-4 text-center">
					{{ authStore.error }}
				</div>

				<p class="text-xs text-gray-400 dark:text-gray-500 text-center mt-6">
					{{ t('login.cookieNotice') }}
				</p>
			</template>
		</Card>
	</div>
</template>
