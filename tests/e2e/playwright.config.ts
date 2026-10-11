import { defineConfig } from "@playwright/test";

// Runs against a running stack (`make demo`). BASE_URL overrides the target.
export default defineConfig({
	testDir: ".",
	timeout: 60_000,
	expect: { timeout: 10_000 },
	forbidOnly: !!process.env.CI,
	workers: process.env.CI ? 1 : undefined,
	reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
	use: {
		baseURL: process.env.BASE_URL ?? "https://localhost:8443",
		ignoreHTTPSErrors: true,
		trace: "retain-on-failure",
	},
	projects: [
		{ name: "app", testIgnore: /restart\.spec\.ts/ },
		// restarts the editor container, so it must not overlap with the other tests
		{ name: "restart", testMatch: /restart\.spec\.ts/, dependencies: ["app"] },
	],
});
