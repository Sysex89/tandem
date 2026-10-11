import { eq, and, or } from "drizzle-orm";
import { FastifyPluginAsyncTypebox } from "@fastify/type-provider-typebox";
import { Type, type Static } from "@sinclair/typebox";
import * as schema from "../db/schema";
import { verifySession } from "../lib/session_helpers";
import { upsertAccount } from "../lib/account_helpers";
import * as argon2 from "argon2";
import { authMeTotal } from "../metrics";

const notesServiceBaseUrl = process.env.NOTES_SERVICE_URL ?? "http://notes:3003";

type NotesExportItem = {
	id: string;
	title: string;
	owner_id: string | null;
	created_at: string;
	updated_at: string;
	state_vector: number[] | null;
};

async function callNotesService(
	path: string,
	method: "GET" | "DELETE",
	userId: string,
): Promise<{ ok: boolean; status: number; body: unknown }> {
	try {
		const response = await fetch(`${notesServiceBaseUrl}${path}`, {
			method,
			headers: {
				"X-User-Id": userId,
			},
		});

		let body: unknown = null;
		const text = await response.text();
		if (text) {
			try {
				body = JSON.parse(text);
			} catch {
				body = text;
			}
		}

		return { ok: response.ok, status: response.status, body };
	} catch (_error) {
		return { ok: false, status: 0, body: { error: "Notes service unreachable." } };
	}
}

/* Schemas for Inputs */
const ChangeLoginSchema = Type.Object({
	loginName: Type.String({ minLength: 3, maxLength: 50 }),
});

const ChangeEmailSchema = Type.Object({
	email: Type.String({ format: "email" }),
});

const ChangePasswordSchema = Type.Object({
	oldPassword: Type.String(),
	newPassword: Type.String({ minLength: 8 }),
});

const ChangeImageSchema = Type.Object({
	imageURL: Type.Union([Type.String({ minLength: 1 }), Type.Null()]),
});

const ResolveUserSchema = Type.Object({
	identifier: Type.String({ minLength: 3 }),
});

type ChangeLoginType = Static<typeof ChangeLoginSchema>;
type ChangeEmailType = Static<typeof ChangeEmailSchema>;
type ChangePasswordType = Static<typeof ChangePasswordSchema>;
type ChangeImageType = Static<typeof ChangeImageSchema>;
type ResolveUserType = Static<typeof ResolveUserSchema>;

/**
 * User Management Routes
 * Handles profile retrieval and (future) profile updates.
 */
export const userManagementRoutes: FastifyPluginAsyncTypebox = async (server) => {
	/* Returns the full user profile (Id, Email, Role, etc.) */
	server.get("/me", async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) {
			authMeTotal.labels("401").inc();
			return reply.status(401).send({ error: "No active session found." });
		}
		// Get User Profile
		const [user] = await server.db.select().from(schema.users).where(eq(schema.users.id, session.userId)).limit(1);
		if (!user) {
			authMeTotal.labels("404").inc();
			return reply.status(404).send({ error: "User profile not found." });
		}
		authMeTotal.labels("200").inc();
		// Check if user has a local password account
		const [localAccount] = await server.db
			.select({ id: schema.accounts.id })
			.from(schema.accounts)
			.where(and(eq(schema.accounts.userId, user.id), eq(schema.accounts.provider, "local")))
			.limit(1);
		// Return sanitized user data
		return {
			authenticated: true,
			user: {
				id: user.id,
				loginName: user.loginName,
				email: user.email,
				role: user.role,
				imageURL: user.imageURL,
				createdAt: user.createdAt,
				hasLocalAuth: !!localAccount,
			},
		};
	});

	/*
	 * GET /resolve: Look up a user by exact email or loginName.
	 * Used by the Frontend to verify identity before creating a share.
	 */
	server.get("/resolve", { schema: { querystring: ResolveUserSchema } }, async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const { identifier } = request.query;

		const [user] = await server.db
			.select({
				id: schema.users.id,
				loginName: schema.users.loginName,
				imageURL: schema.users.imageURL,
			})
			.from(schema.users)
			.where(or(eq(schema.users.email, identifier), eq(schema.users.loginName, identifier)))
			.limit(1);

		if (!user) {
			return reply.status(404).send({ error: "No user found with that email or username." });
		}

		return { user: user };
	});

	/*
	 * GET /users: Returns all registered users (id + loginName).
	 * Used by the frontend share dialog to list/search users.
	 */
	server.get("/users", async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const users = await server.db
			.select({
				id: schema.users.id,
				loginName: schema.users.loginName,
				imageURL: schema.users.imageURL,
			})
			.from(schema.users);

		return { users };
	});

	/* PATCH /change-login: Updates the public identity (loginName). */
	server.patch("/change-login", { schema: { body: ChangeLoginSchema } }, async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const { loginName } = request.body;

		try {
			await server.db.update(schema.users).set({ loginName }).where(eq(schema.users.id, session.userId));
			return { message: "Username updated successfully.", user: { loginName } };
		} catch (err: any) {
			const pgCode = err.code ?? err.cause?.code;
			if (pgCode === "23505") {
				return reply.status(409).send({ error: "Username already taken." });
			}
			server.log.error(err);
			return reply.status(500).send({ error: "Failed to update username." });
		}
	});

	/* PATCH /change-email: Updates the private identity (email).*/
	server.patch("/change-email", { schema: { body: ChangeEmailSchema } }, async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const { email } = request.body;
		try {
			await server.db.update(schema.users).set({ email }).where(eq(schema.users.id, session.userId));
			return { message: "Email updated successfully.", user: { email } };
		} catch (err: any) {
			const pgCode = err.code ?? err.cause?.code;
			if (pgCode === "23505") {
				return reply.status(409).send({ error: "Email already in use." });
			}
			server.log.error(err);
			return reply.status(500).send({ error: "Failed to update email." });
		}
	});

	/* PATCH /change-image: Sets or clears the profile picture URL. */
	server.patch("/change-image", { schema: { body: ChangeImageSchema } }, async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const { imageURL } = request.body;
		await server.db.update(schema.users).set({ imageURL }).where(eq(schema.users.id, session.userId));
		return { message: "Profile picture updated.", user: { imageURL } };
	});

	/**
	 * POST /change-password: Updates the password for the current user.
	 * Look for a 'local' provider account in the accounts table.
	 */
	server.post("/change-password", { schema: { body: ChangePasswordSchema } }, async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const { oldPassword, newPassword } = request.body;

		// Get User Profile to check for email
		const [user] = await server.db.select().from(schema.users).where(eq(schema.users.id, session.userId)).limit(1);

		if (!user) return reply.status(404).send({ error: "User not found." });
		if (!user.email) {
			return reply.status(400).send({ error: "Please add an email before adding a login password." });
		}

		// Find the 'local' account for this user
		const [account] = await server.db
			.select()
			.from(schema.accounts)
			.where(and(eq(schema.accounts.userId, session.userId), eq(schema.accounts.provider, "local")))
			.limit(1);

		// Handle Identity Upgrading (No local account yet)
		if (!account || !account.passwordHash) {
			const newHash = await argon2.hash(newPassword);
			await upsertAccount(server, {
				userId: session.userId,
				provider: "local",
				providerAccountId: session.userId,
				passwordHash: newHash,
			});
			return { message: "Local account created and password set." };
		}

		// Standard Password Change (Verify current password)
		const isMatch = await argon2.verify(account.passwordHash, oldPassword);
		if (!isMatch) {
			return reply.status(401).send({ error: "Incorrect current password." });
		}

		const newHash = await argon2.hash(newPassword);
		await server.db
			.update(schema.accounts)
			.set({ passwordHash: newHash })
			.where(and(
				eq(schema.accounts.userId, session.userId),
				eq(schema.accounts.provider, "local")
			));

		return { message: "Password updated successfully." };
	});

	server.get("/export-data", async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const [user] = await server.db.select().from(schema.users).where(eq(schema.users.id, session.userId)).limit(1);
		if (!user) return reply.status(404).send({ error: "User not found." });

		const notesResponse = await callNotesService("/api/notes/export", "GET", session.userId);
		if (!notesResponse.ok) {
			return reply.status(502).send({
				error: "Failed to export notes from notes service.",
				upstreamStatus: notesResponse.status || undefined,
			});
		}

		const notes = Array.isArray(notesResponse.body) ? (notesResponse.body as NotesExportItem[]) : [];

		const accounts = await server.db
			.select({
				provider: schema.accounts.provider,
				providerAccountId: schema.accounts.providerAccountId,
			})
			.from(schema.accounts)
			.where(eq(schema.accounts.userId, session.userId));

		const sessions = await server.db
			.select({
				expiresAt: schema.sessions.expiresAt,
				userAgent: schema.sessions.userAgent,
				ipAddress: schema.sessions.ipAddress,
			})
			.from(schema.sessions)
			.where(eq(schema.sessions.userId, session.userId));

		return {
			exportedAt: new Date().toISOString(),
			user: {
				id: user.id,
				loginName: user.loginName,
				email: user.email,
				role: user.role,
				status: user.status,
				imageURL: user.imageURL,
				createdAt: user.createdAt,
			},
			linkedAccounts: accounts.map((a) => ({
				provider: a.provider,
				providerAccountId: a.providerAccountId,
			})),
			activeSessions: sessions.map((s) => ({
				expiresAt: s.expiresAt,
				userAgent: s.userAgent,
				ipAddress: s.ipAddress,
			})),
			notes,
		};
	});

	server.delete("/delete-account", async (request, reply) => {
		const session = await verifySession(request, server.db);
		if (!session) return reply.status(401).send({ error: "Unauthorized" });

		const deleteNotesResponse = await callNotesService("/api/notes/by-owner", "DELETE", session.userId);
		if (!deleteNotesResponse.ok && deleteNotesResponse.status !== 404) {
			return reply.status(502).send({
				error: "Failed to delete user notes.",
				upstreamStatus: deleteNotesResponse.status || undefined,
			});
		}

		await server.db.delete(schema.users).where(eq(schema.users.id, session.userId));
		reply.clearCookie("session_id", { path: "/" });

		return { message: "Account deleted successfully." };
	});
};

// - Add Password
// - Lost Password
// - Delete Account
