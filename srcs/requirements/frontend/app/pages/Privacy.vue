<script setup lang="ts">
import { useUiI18n } from '~/composables/useUiI18n';
const { t } = useUiI18n();
const contactEmail = 'myakoven@student.42berlin.de';
</script>

<template>
	<div class="max-w-3xl mx-auto px-6 py-10 overflow-y-auto h-full text-surface-800 dark:text-surface-200">
		<h1 class="text-2xl font-bold mb-1">{{ t('legal.privacy.title') }}</h1>
		<p class="text-sm text-surface-500 mb-8">{{ t('legal.lastUpdated') }}: 17/04/2026</p>
		<p class="mb-6">{{ t('privacy.intro') }}</p>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s1.title') }}</h2>
			<h3 class="font-medium mt-3 mb-1">{{ t('privacy.s1_1.title') }}</h3>
			<p>{{ t('privacy.s1_1.body') }}</p>
			<h3 class="font-medium mt-3 mb-1">{{ t('privacy.s1_2.title') }}</h3>
			<p>{{ t('privacy.s1_2.body') }}</p>
			<h3 class="font-medium mt-3 mb-1">{{ t('privacy.s1_3.title') }}</h3>
			<p>{{ t('privacy.s1_3.body') }}</p>
			<h3 class="font-medium mt-3 mb-1">{{ t('privacy.s1_5.title') }}</h3>
			<p class="mb-2">{{ t('privacy.s1_5.intro') }}</p>
			<ul class="list-disc pl-6 mb-2 space-y-1">
				<li><strong>session_id</strong>: {{ t('privacy.s1_5.session') }}</li>
				<li><strong>lang</strong>: {{ t('privacy.s1_5.lang') }}</li>
			</ul>
			<p>{{ t('privacy.s1_5.closing') }}</p>
			<h3 class="font-medium mt-3 mb-1">{{ t('privacy.s1_6.title') }}</h3>
			<p>{{ t('privacy.s1_6.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s2.title') }}</h2>
			<p>{{ t('privacy.s2.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s3.title') }}</h2>
			<ul class="list-disc pl-6 space-y-1">
				<li>{{ t('privacy.s3.account') }}</li>
				<li>{{ t('privacy.s3.session') }}</li>
				<li>{{ t('privacy.s3.content') }}</li>
				<li>{{ t('privacy.s3.metrics') }}</li>
			</ul>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s4.title') }}</h2>
			<p>{{ t('privacy.s4.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s5.title') }}</h2>
			<p>{{ t('privacy.s5.github') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s6.title') }}</h2>
			<p>{{ t('privacy.s6.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s7.title') }}</h2>
			<ul class="list-disc pl-6 space-y-1">
				<li>{{ t('privacy.s7.sessions') }}</li>
				<li>{{ t('privacy.s7.account') }}</li>
				<li>{{ t('privacy.s7.logs') }}</li>
			</ul>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s8.title') }}</h2>
			<p class="mb-2">{{ t('privacy.s8.intro') }}</p>
			<ul class="list-disc pl-6 space-y-1">
				<li>{{ t('privacy.s8.profile') }}</li>
				<li>{{ t('privacy.s8.providers') }}</li>
				<li>{{ t('privacy.s8.sessions') }}</li>
				<li>{{ t('privacy.s8.notes') }}</li>
			</ul>
			<p class="mt-2">{{ t('privacy.s8.format') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s9.title') }}</h2>
			<p class="mb-2">{{ t('privacy.s9.intro') }}</p>
			<ul class="list-disc pl-6 space-y-1">
				<li>{{ t('privacy.s9.sessions') }}</li>
				<li>{{ t('privacy.s9.notes') }}</li>
			</ul>
			<p class="mt-2">{{ t('privacy.s9.permanent') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s10.title') }}</h2>
			<p>{{ t('privacy.s10.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s11.title') }}</h2>
			<p>{{ t('privacy.s11.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('privacy.s12.title') }}</h2>
			<p>{{ t('privacy.s12.body') }} <a :href="'mailto:' + contactEmail" class="text-primary-500 hover:underline">{{ contactEmail }}</a>.</p>
		</section>
	</div>
</template>
