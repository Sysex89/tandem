import { defineConfig } from "vitest/config";

// Only run the TypeScript sources; `pnpm build` also emits compiled copies into dist/.
export default defineConfig({
	test: {
		include: ["src/**/*.test.ts"],
	},
});
