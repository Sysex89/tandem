<script setup lang="ts">
import { useUiI18n } from '~/composables/useUiI18n';
const { t } = useUiI18n();
const contactEmail = 'myakoven@student.42berlin.de';
</script>

<template>
	<div class="max-w-3xl mx-auto px-6 py-10 overflow-y-auto h-full text-surface-800 dark:text-surface-200">
		<h1 class="text-2xl font-bold mb-1">{{ t('legal.terms.title') }}</h1>
		<p class="text-sm text-surface-500 mb-8">{{ t('legal.lastUpdated') }}: 17/04/2026</p>
		<p class="mb-6">{{ t('terms.intro') }}</p>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s1.title') }}</h2>
			<ul class="list-disc pl-6 space-y-1">
				<li>{{ t('terms.s1.service') }}</li>
				<li>{{ t('terms.s1.user') }}</li>
				<li>{{ t('terms.s1.content') }}</li>
				<li>{{ t('terms.s1.we') }}</li>
			</ul>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s2.title') }}</h2>
			<p class="mb-2">2.1 {{ t('terms.s2.1') }}</p>
			<p class="mb-2">2.2 {{ t('terms.s2.2') }}</p>
			<p class="mb-2">2.3 {{ t('terms.s2.3') }}</p>
			<p class="mb-2">2.4 {{ t('terms.s2.4') }}</p>
			<p>2.5 {{ t('terms.s2.5') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s3.title') }}</h2>
			<p class="mb-2">3.1 <strong>{{ t('terms.s3.1.label') }}</strong> {{ t('terms.s3.1') }}</p>
			<p class="mb-2">3.2 <strong>{{ t('terms.s3.2.label') }}</strong> {{ t('terms.s3.2') }}</p>
			<p class="mb-2">3.3 <strong>{{ t('terms.s3.3.label') }}</strong> {{ t('terms.s3.3') }}</p>
			<p>3.4 <strong>{{ t('terms.s3.5.label') }}</strong> {{ t('terms.s3.5') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s4.title') }}</h2>
			<p class="mb-2">{{ t('terms.s4.intro') }}</p>
			<ul class="list-[lower-alpha] pl-6 space-y-1">
				<li>{{ t('terms.s4.a') }}</li>
				<li>{{ t('terms.s4.b') }}</li>
				<li>{{ t('terms.s4.c') }}</li>
				<li>{{ t('terms.s4.d') }}</li>
				<li>{{ t('terms.s4.e') }}</li>
				<li>{{ t('terms.s4.f') }}</li>
			</ul>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s5.title') }}</h2>
			<p class="mb-2">5.1 {{ t('terms.s5.1') }}</p>
			<p class="mb-2">5.2 {{ t('terms.s5.2') }}</p>
			<p class="mb-2">5.3 {{ t('terms.s5.3') }}</p>
			<p>5.4 {{ t('terms.s5.4') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s6.title') }}</h2>
			<p class="mb-2">6.1 {{ t('terms.s6.1') }}</p>
			<p>6.2 {{ t('terms.s6.2') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s7.title') }}</h2>
			<p class="mb-2">7.1 {{ t('terms.s7.1') }}</p>
			<p class="mb-2">7.2 {{ t('terms.s7.2') }}</p>
			<p>7.3 {{ t('terms.s7.3') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s8.title') }}</h2>
			<p>{{ t('terms.s8.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s9.title') }}</h2>
			<p>{{ t('terms.s9.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s10.title') }}</h2>
			<p>{{ t('terms.s10.body') }} <NuxtLink to="/privacy" class="text-primary-500 hover:underline">{{ t('legal.privacy.title') }}</NuxtLink>.</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s11.title') }}</h2>
			<p>{{ t('terms.s11.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s12.title') }}</h2>
			<p>{{ t('terms.s12.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s13.title') }}</h2>
			<p>{{ t('terms.s13.body') }}</p>
		</section>

		<section class="mb-6">
			<h2 class="text-lg font-semibold mb-2">{{ t('terms.s14.title') }}</h2>
			<p>{{ t('terms.s14.body') }} <a :href="'mailto:' + contactEmail" class="text-primary-500 hover:underline">{{ contactEmail }}</a>.</p>
		</section>
	</div>
</template>
