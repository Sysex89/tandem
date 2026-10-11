import { expect, test } from "@playwright/test";
import { createNote, deleteNote, openNote, signIn, typeAtEnd, type User } from "./helpers";

let alice: User;
let bob: User;

test.beforeAll(async () => {
	alice = await signIn("e2e_alice");
	bob = await signIn("e2e_bob");
});

test("edits sync live in both directions and show the other user's cursor", async ({ browser }) => {
	const { noteId, title } = await createNote(alice, bob, "Edit");
	const alicePage = await openNote(browser, alice, title);
	const bobPage = await openNote(browser, bob, title);

	await typeAtEnd(alicePage, "Hello from alice.");
	await expect(bobPage.locator(".ProseMirror")).toContainText("Hello from alice.");

	await typeAtEnd(bobPage, " And hi from bob.");
	await expect(alicePage.locator(".ProseMirror")).toContainText("Hello from alice. And hi from bob.");

	// Each user sees the other's caret, labelled with their name.
	await expect(alicePage.locator(".collaboration-carets__label", { hasText: "e2e_bob" })).toBeAttached();
	await expect(bobPage.locator(".collaboration-carets__label", { hasText: "e2e_alice" })).toBeAttached();
	await expect(alicePage.getByText("2 collaborators")).toBeVisible();

	await deleteNote(alice, noteId);
});

test("content is persisted after everyone leaves", async ({ browser }) => {
	const { noteId, title } = await createNote(alice, bob, "Edit");
	const alicePage = await openNote(browser, alice, title);
	const bobPage = await openNote(browser, bob, title);

	await typeAtEnd(alicePage, "Saved by alice.");
	await typeAtEnd(bobPage, " Saved by bob.");
	await expect(alicePage.locator(".ProseMirror")).toContainText("Saved by alice. Saved by bob.");

	// The editor service saves the document when the last client disconnects.
	await alicePage.context().close();
	await bobPage.context().close();

	await expect(async () => {
		const reopened = await openNote(browser, alice, title);
		try {
			await expect(reopened.locator(".ProseMirror")).toContainText("Saved by alice. Saved by bob.", {
				timeout: 2_000,
			});
		} finally {
			await reopened.context().close();
		}
	}).toPass({ timeout: 20_000 });

	await deleteNote(alice, noteId);
});

test("the title syncs to collaborators and is stored for the owner's note list", async ({ browser }) => {
	const { noteId, title } = await createNote(alice, bob, "Edit");
	const alicePage = await openNote(browser, alice, title);
	const bobPage = await openNote(browser, bob, title);

	const renamed = `${title} renamed`;
	await alicePage.locator(".editor-title").fill(renamed);
	await expect(bobPage.locator(".editor-title")).toHaveValue(renamed);

	// The title is saved to the notes service after a short debounce.
	await expect(async () => {
		const note = await (await alice.api.get(`/api/notes/${noteId}`)).json();
		expect(note.title).toBe(renamed);
	}).toPass({ timeout: 10_000 });

	await deleteNote(alice, noteId);
});
