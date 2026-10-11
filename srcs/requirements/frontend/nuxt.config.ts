import tailwindcss from "@tailwindcss/vite";

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
	compatibilityDate: "2025-07-15",
	devtools: { enabled: true },
	modules: ["@primevue/nuxt-module", "@pinia/nuxt", "@vueuse/nuxt"],
	css: ["~/assets/base.css"],
	app: {
		head: {
			title: "Tandem",
			link: [{ rel: "icon", type: "image/svg+xml", href: "/favicon.svg" }],
			script: [
				{
					innerHTML: `(function(){var s=localStorage.getItem('theme');var d=s?s==='dark':window.matchMedia('(prefers-color-scheme:dark)').matches;if(d)document.documentElement.classList.add('dark')})()`,
					type: "text/javascript",
				},
			],
		},
	},
	vite: {
		plugins: [tailwindcss() as any],
		server: {
			watch: {
				usePolling: true,
				interval: 1000, // ms
			},
		},
	},
	components: [
		{ path: '~/components', pathPrefix: false }
	],
	future: {
		compatibilityVersion: 4,
	},
});
// https://nuxt.com/modules contains modules
