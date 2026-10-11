import { expect, request, type APIRequestContext, type Browser, type Page } from "@playwright/test";

export const BASE_URL = process.env.BASE_URL ?? "https://localhost:8443";
const PASSWORD = "tandem-e2e";

export type User = { name: string; id: string; api: APIRequestContext };

/** Registers the user, or logs in if they already exist, and returns an authenticated API client. */
export async function signIn(name: string): Promise<User> {
	const api = await request.newContext({ baseURL: BASE_URL, ignoreHTTPSErrors: true });
	const register = await api.post("/api/auth/register", {
		data: { loginName: name, email: `${name}@e2e.local`, password: PASSWORD },
	});
	if (register.status() === 409) {
		const login = await api.post("/api/auth/login", { data: { identifier: name, password: PASSWORD } });
		expect(login.status(), `login as ${name}`).toBe(200);
	} else {
		expect(register.status(), `register ${name}`).toBe(200);
	}
	const me = await api.get("/api/auth/me");
	expect(me.status()).toBe(200);
	return { name, id: (await me.json()).user.id, api };
}

/** Creates a uniquely titled note owned by `owner`, optionally shared with `guest`. */
export async function createNote(owner: User, guest?: User, role: "View" | "Edit" = "Edit") {
	const title = `e2e ${Date.now()} ${Math.random().toString(36).slice(2, 6)}`;
	const created = await owner.api.post("/api/notes", { data: { title } });
	expect(created.status()).toBe(200);
	const noteId: string = (await created.json()).id;
	let shareId: string | undefined;
	if (guest) {
		const share = await owner.api.post("/api/notes/collab/", {
			data: { note_id: noteId, guest_id: guest.id, role },
		});
		expect(share.ok(), `share note with ${guest.name}`).toBeTruthy();
		shareId = (await share.json()).id;
	}
	return { noteId, shareId, title };
}

export async function deleteNote(owner: User, noteId: string) {
	await owner.api.delete(`/api/notes/${noteId}`);
}

/** Opens the Notes page as `user` in a fresh browser context and selects the note by title. */
export async function openNote(
	browser: Browser,
	user: User,
	title: string,
	{ editable = true } = {},
): Promise<Page> {
	const context = await browser.newContext({
		baseURL: BASE_URL,
		ignoreHTTPSErrors: true,
		storageState: await user.api.storageState(),
	});
	const page = await context.newPage();
	await page.goto("/notes");
	await page.getByRole("option", { name: title }).click();
	await expect(page.locator(".ProseMirror")).toBeVisible();
	await expect(page.locator(".ProseMirror")).toHaveAttribute("contenteditable", String(editable));
	return page;
}

/** Types at the end of the document. */
export async function typeAtEnd(page: Page, text: string) {
	const editor = page.locator(".ProseMirror");
	await editor.click();
	await editor.evaluate((el) => {
		const range = document.createRange();
		range.selectNodeContents(el);
		range.collapse(false);
		getSelection()?.removeAllRanges();
		getSelection()?.addRange(range);
	});
	await page.keyboard.type(text);
}

/** Tries to open the collaboration WebSocket for a note from inside `page` (uses its cookies). */
export function tryWebSocket(page: Page, noteId: string): Promise<"open" | "rejected"> {
	return page.evaluate(
		(id) =>
			new Promise<"open" | "rejected">((resolve) => {
				const ws = new WebSocket(`wss://${location.host}/ws/${id}`);
				ws.onopen = () => {
					ws.close();
					resolve("open");
				};
				ws.onerror = () => resolve("rejected");
			}),
		noteId,
	);
}
