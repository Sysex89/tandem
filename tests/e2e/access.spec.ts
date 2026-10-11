import { expect, test } from "@playwright/test";
import { BASE_URL, createNote, deleteNote, openNote, signIn, tryWebSocket, typeAtEnd, type User } from "./helpers";

let alice: User;
let bob: User;
let carol: User;

test.beforeAll(async () => {
	alice = await signIn("e2e_alice");
	bob = await signIn("e2e_bob");
	carol = await signIn("e2e_carol"); // never gets a share
});

test.describe("nginx authentication gate", () => {
	test("protected API routes reject requests without a session", async ({ playwright }) => {
		const anonymous = await playwright.request.newContext({ baseURL: BASE_URL, ignoreHTTPSErrors: true });
		expect((await anonymous.get("/api/notes")).status()).toBe(401);
		await anonymous.dispose();
	});

	test("a client-supplied X-User-Id header is ignored", async ({ playwright }) => {
		const { noteId } = await createNote(alice);

		// Without a session, claiming to be alice gets nowhere.
		const anonymous = await playwright.request.newContext({ baseURL: BASE_URL, ignoreHTTPSErrors: true });
		const spoofed = await anonymous.get("/api/notes", { headers: { "X-User-Id": alice.id } });
		expect(spoofed.status()).toBe(401);
		await anonymous.dispose();

		// With carol's session, nginx overwrites the header with carol's verified id.
		const asCarol = await carol.api.get("/api/notes", { headers: { "X-User-Id": alice.id } });
		expect(asCarol.status()).toBe(200);
		const ids = (await asCarol.json()).map((n: { id: string }) => n.id);
		expect(ids).not.toContain(noteId);

		await deleteNote(alice, noteId);
	});
});

test.describe("note access", () => {
	test("a user without a share cannot read the note or join its editing session", async ({ browser }) => {
		const { noteId } = await createNote(alice, bob, "Edit");

		expect((await carol.api.get(`/api/notes/${noteId}`)).status()).toBe(404);

		const page = await (
			await browser.newContext({ ignoreHTTPSErrors: true, storageState: await carol.api.storageState() })
		).newPage();
		await page.goto(`${BASE_URL}/notes`);
		expect(await tryWebSocket(page, noteId)).toBe("rejected");

		await deleteNote(alice, noteId);
	});

	test("revoking a share removes access", async ({ browser }) => {
		const { noteId, shareId } = await createNote(alice, bob, "Edit");
		const page = await (
			await browser.newContext({ ignoreHTTPSErrors: true, storageState: await bob.api.storageState() })
		).newPage();
		await page.goto(`${BASE_URL}/notes`);
		expect(await tryWebSocket(page, noteId)).toBe("open");

		const revoke = await alice.api.delete(`/api/notes/collab/revoke/${shareId}`);
		expect(revoke.ok()).toBeTruthy();

		expect(await tryWebSocket(page, noteId)).toBe("rejected");
		const received = await (await bob.api.get("/api/notes/collab/received")).json();
		expect(received.map((s: { note_id: string }) => s.note_id)).not.toContain(noteId);

		await deleteNote(alice, noteId);
	});

	// Known bug: the editor service only checks *whether* a share exists, not its
	// role, and the frontend does not make the editor read-only, so "View" guests
	// can edit. Remove test.fail() once view-only access is enforced.
	test("a view-only guest cannot change the document", async ({ browser }) => {
		test.fail();
		const { noteId, title } = await createNote(alice, bob, "View");
		const alicePage = await openNote(browser, alice, title);
		await typeAtEnd(alicePage, "Owner text.");

		const bobPage = await openNote(browser, bob, title).catch(() => null);
		if (bobPage) await typeAtEnd(bobPage, " Viewer edit.");
		await alicePage.waitForTimeout(1_500);
		await expect(alicePage.locator(".ProseMirror")).not.toContainText("Viewer edit.", { timeout: 1_000 });

		await deleteNote(alice, noteId);
	});
});
