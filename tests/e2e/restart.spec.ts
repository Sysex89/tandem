import { expect, test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { createNote, deleteNote, openNote, signIn, typeAtEnd } from "./helpers";

// Restarts the editor container, so this file runs in its own project after all the others.
test("restarting the editor is quick and keeps edits made just before it", async ({ browser }) => {
	const alice = await signIn("e2e_alice");
	const bob = await signIn("e2e_bob");
	const { noteId, title } = await createNote(alice, bob, "Edit");
	const alicePage = await openNote(browser, alice, title);
	const bobPage = await openNote(browser, bob, title);

	await typeAtEnd(alicePage, "Typed right before a deploy.");
	await expect(bobPage.locator(".ProseMirror")).toContainText("Typed right before a deploy."); // the server has it

	// Only the server's copy may count: stop both pages from reconnecting and re-sending their state.
	for (const page of [alicePage, bobPage]) await page.routeWebSocket(/\/ws\//, (ws) => ws.close());

	const started = Date.now();
	execFileSync("docker", ["restart", "editor"]);
	// without a SIGTERM handler, Docker waits out its 10 s stop timeout and then kills the editor
	expect(Date.now() - started, "editor restart time (ms)").toBeLessThan(5_000);

	await alicePage.context().close();
	await bobPage.context().close();
	const reopened = await openNote(browser, alice, title);
	await expect(reopened.locator(".ProseMirror")).toContainText("Typed right before a deploy.");

	await deleteNote(alice, noteId);
});
