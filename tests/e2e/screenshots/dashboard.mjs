// Captures the README screenshot of the editor's Grafana dashboard.
// Requires `make demo`; generate some traffic first (e.g. `make e2e`) so the panels have data.
//
//   cd tests/e2e && pnpm install && pnpm exec playwright install chromium
//   node screenshots/dashboard.mjs [output.png]   (run from tests/e2e)
import { chromium } from '@playwright/test';
import { readFileSync } from 'node:fs';

const BASE = process.env.BASE ?? 'https://localhost:8443';
const OUT = process.argv[2] ?? '../../DOCS/img/editor-dashboard.png';
const FROM = process.env.FROM ?? 'now-15m'; // e.g. FROM=now-1h to cover a load test
const secret = (name) => readFileSync(new URL(`../../../srcs/secrets/${name}`, import.meta.url), 'utf8').trim();

const browser = await chromium.launch();
const page = await (
	await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 1600, height: 1300 }, deviceScaleFactor: 2 })
).newPage();

await page.goto(`${BASE}/grafana/login`);
await page.locator('input[name="user"]').fill(secret('grafana_admin_user'));
await page.locator('input[name="password"]').fill(secret('grafana_admin_password'));
await page.getByRole('button', { name: /log in/i }).click();
await page.waitForURL((url) => !url.pathname.endsWith('/login'));

// Tall enough that Grafana renders every panel (it lazy-loads panels below the fold).
await page.goto(`${BASE}/grafana/d/editor-metrics?kiosk&from=${FROM}&to=now`);
await page.getByText('Revocations and title syncs').waitFor();
await page.waitForTimeout(5000); // let the queries come back and the charts draw
await page.screenshot({ path: OUT });
await browser.close();
console.log(`Saved ${OUT}`);
