// Captures the README screenshot: alice and bob editing the same note, each
// seeing the other's live cursor. Requires `make demo` to be running.
//
//   cd tests/e2e && pnpm install && pnpm exec playwright install chromium
//   node screenshots/collaboration.mjs [output.png]   (run from tests/e2e)
import { chromium } from '@playwright/test';
import { readFileSync } from 'node:fs';

const BASE = process.env.BASE ?? 'https://localhost:8443';
const OUT = process.argv[2] ?? '../../DOCS/img/collaboration.png';
const VIEWPORT = { width: 1100, height: 720 };

const browser = await chromium.launch();

async function openNote(name) {
	const context = await browser.newContext({ ignoreHTTPSErrors: true, viewport: VIEWPORT, deviceScaleFactor: 2 });
	const page = await context.newPage();
	await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
	await page.getByPlaceholder('Username or Email').fill(name);
	await page.getByPlaceholder('Password').first().fill('mycelium-demo');
	await page.getByRole('button', { name: 'Sign In' }).click();
	await page.waitForURL('**/notes');
	await page.getByRole('option', { name: /Project kickoff/ }).click();
	await page.locator('.ProseMirror').waitFor();
	await page.waitForTimeout(1500); // let the WebSocket sync settle
	return page;
}

const alice = await openNote('alice');
const bob = await openNote('bob');

// Start from an empty document so the shot is reproducible.
await alice.locator('.editor-title').fill('Project kickoff');
const editor = alice.locator('.ProseMirror');
await editor.click();
await alice.keyboard.press('Control+A');
await alice.keyboard.press('Backspace');

const type = (page, text) => page.keyboard.type(text, { delay: 5 });
await type(alice, 'Goals for the first sprint:\n');
await type(alice, '- Share notes with view or edit access\n');
await type(alice, 'Live cursors for every collaborator\n');
await type(alice, 'Dashboards for each service\n\n');
await type(alice, 'Open question: how should we handle offline edits?');
await bob.waitForTimeout(1000);

// Put bob's caret at the end of alice's question (End/Ctrl+End are unreliable here).
const question = bob.locator('.ProseMirror p', { hasText: 'Open question' });
await question.click();
await question.evaluate((el) => {
	const range = document.createRange();
	range.selectNodeContents(el);
	range.collapse(false);
	getSelection().removeAllRanges();
	getSelection().addRange(range);
});
await bob.waitForTimeout(100);
await type(bob, '\nYjs merges them when the client reconnects. I can write a test for it');

// Caret labels fade after 2s of inactivity, so move both carets right before capturing.
await alice.locator('.ProseMirror li').nth(1).click();
await alice.keyboard.press('End');
await bob.keyboard.press('ArrowLeft');
await bob.keyboard.press('ArrowRight');
await alice.waitForTimeout(400);

const [a, b] = await Promise.all([alice.screenshot(), bob.screenshot()]);

// Compose the two windows side by side on a neutral background.
const img = (buf) => `data:image/png;base64,${buf.toString('base64')}`;
const composer = await (await browser.newContext({ viewport: { width: 2 * VIEWPORT.width + 120, height: VIEWPORT.height + 140 }, deviceScaleFactor: 1 })).newPage();
await composer.setContent(`
	<body style="margin:0;background:#e7ece6;display:flex;gap:40px;justify-content:center;align-items:center;height:100vh;font:600 22px system-ui;color:#4b5b4a">
		${[['alice', a], ['bob', b]].map(([n, buf]) => `
			<figure style="margin:0">
				<figcaption style="margin-bottom:12px">${n}'s browser</figcaption>
				<img src="${img(buf)}" style="width:${VIEWPORT.width}px;border-radius:10px;box-shadow:0 10px 30px rgba(0,0,0,.18)">
			</figure>`).join('')}
	</body>`);
await composer.screenshot({ path: OUT });
await browser.close();
console.log(`Saved ${OUT}`);
