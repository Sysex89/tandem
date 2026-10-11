<script setup lang="ts">
import { useUiI18n } from '~/composables/useUiI18n';
const { t } = useUiI18n();
</script>

<template>
	<footer dir="ltr" class="flex items-center justify-center gap-4 px-4 py-1.5 text-xs text-surface-400 dark:text-surface-500 border-t border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-900 shrink-0">
		<span>&copy; {{ new Date().getFullYear() }} Tandem</span>
		<span class="text-surface-300 dark:text-surface-600">|</span>
		<NuxtLink to="/privacy" class="hover:text-primary-500 transition-colors">{{ t('legal.privacy.title') }}</NuxtLink>
		<span class="text-surface-300 dark:text-surface-600">|</span>
		<NuxtLink to="/terms" class="hover:text-primary-500 transition-colors">{{ t('legal.terms.title') }}</NuxtLink>
	</footer>
</template>
