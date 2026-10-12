import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { buildServer, type AppConfig } from "../app";
import { databaseUrl } from "../db/connections";
import { sql } from "drizzle-orm";

describe("Auth Routes (Integration)", () => {
	// We need a server instance for our tests
	let server: Awaited<ReturnType<typeof buildServer>>;

	// Helper to print Logs
	function logInteraction(testName: string, req: any, res: any) {
		console.log(`\n🔹 [Test: ${testName}]`);
		console.log(`   Request: ${req.method} ${req.url}`);
		if (req.payload) console.log(`   Payload:`, JSON.stringify(req.payload));
		console.log(`   Response: Status ${res.statusCode}`);
		if (res.headers.location) console.log(`   Redirect To: ${res.headers.location}`);
		if (res.body) console.log(`   Body: ${res.body}`);
		console.log("------------------------------------------");
	}

	// Setup: Build the server before running tests
	beforeAll(async () => {
		// Log the DB URL to verify we are targeting the right environment, with the
		// password masked: test output ends up in CI logs.
		console.log("Test Environment DB:", databaseUrl.replace(/\/\/([^:/]+):.*@/, "//$1:***@"));

		const config: AppConfig = {
			databaseUrl: databaseUrl, // Using the dev DB for now (Caution: this writes real data)
			githubClientId: "test_client_id",
			githubClientSecret: "test_client_secret",
			sessionSecret: "a_very_long_test_secret_key_that_is_32_bytes",
			callbackUri: "http://localhost:8080/cb",
			frontendUrl: "http://localhost:8080",
			runMigrations: true, // No-op when already migrated; lets the suite run on a fresh database
		};

		server = await buildServer(config);
		// Pre-flight cleanup: Ensure test user doesn't exist from a previous crash
		await server.db.execute(sql`DELETE FROM users WHERE email IN ('bob@example.com', 'race@example.com')`);
	});

	// Note: Proper teardown happens in the final afterAll block below

	it("should register a new user successfully", async () => {
		const req = {
			method: "POST" as const,
			url: "/register",
			headers: { host: "localhost:8080" },
			payload: {
				loginName: "testuser_bob",
				email: "bob@example.com",
				password: "securePassword123!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Register Success", req, response);

		expect(response.statusCode).toBe(200);
		expect(response.json()).toEqual({ success: true });
		// Use regex to check for the session cookie
		const setCookie = response.headers["set-cookie"];
		expect(setCookie).toBeDefined();

		// Handle potential array or string (fastify inject behavior)
		const cookieVal = Array.isArray(setCookie) ? setCookie[0] : setCookie;
		expect(cookieVal).toMatch(/session_id=/);
	});

	it("should fail to register with duplicate email", async () => {
		const req = {
			method: "POST" as const,
			url: "/register",
			payload: {
				loginName: "testuser_bob_2", // Different name
				email: "bob@example.com", // SAME email
				password: "securePassword123!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Register Duplicate Email", req, response);

		expect(response.statusCode).toBe(409);
		expect(response.json()).toEqual({ error: "Email already registered." });
	});

	it("should login successfully with correct credentials", async () => {
		const req = {
			method: "POST" as const,
			url: "/login",
			headers: { host: "localhost:8080" },
			payload: {
				identifier: "bob@example.com",
				password: "securePassword123!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Login Success", req, response);

		expect(response.statusCode).toBe(200);
		expect(response.json()).toEqual({ success: true });
		expect(response.headers["set-cookie"]).toBeDefined();
	});

	it("should reject login with wrong password", async () => {
		const req = {
			method: "POST" as const,
			url: "/login",
			payload: {
				identifier: "bob@example.com",
				password: "wrongPassword!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Login Wrong Password", req, response);
		expect(response.statusCode).toBe(401);
		expect(response.json()).toEqual({ error: "Invalid credentials." });
	});

	it("should reject login for a non-existent user", async () => {
		const req = {
			method: "POST" as const,
			url: "/login",
			payload: {
				identifier: "nobody@example.com",
				password: "somePassword123!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Login Non-existent User", req, response);
		expect(response.statusCode).toBe(401);
		expect(response.json()).toEqual({ error: "Invalid credentials." });
	});

	it("should fail to register with duplicate loginName", async () => {
		const req = {
			method: "POST" as const,
			url: "/register",
			payload: {
				loginName: "testuser_bob", // Same name as existing user
				email: "different@example.com",
				password: "securePassword123!",
			},
		};
		const response = await server.inject(req);

		logInteraction("Register Duplicate LoginName", req, response);
		expect(response.statusCode).toBe(409);
		expect(response.json()).toEqual({ error: "Username already taken." });
	});

	// -- Session Tests --

	// Helper: log in as bob and return the raw cookie string for use in request headers
	async function loginBob(): Promise<string> {
		const res = await server.inject({
			method: "POST",
			url: "/login",
			headers: { host: "localhost:8080" },
			payload: { identifier: "bob@example.com", password: "securePassword123!" },
		});
		const setCookie = res.headers["set-cookie"];
		const raw = Array.isArray(setCookie) ? setCookie[0] : (setCookie ?? "");
		return raw.split(";")[0]; // "session_id=s%3A..."
	}

	it("should return 200 and X-User-Id header for a valid session on /verify", async () => {
		const cookie = await loginBob();
		const req = {
			method: "GET" as const,
			url: "/verify",
			headers: { cookie },
		};
		const response = await server.inject(req);

		logInteraction("Verify Valid Session", req, response);
		expect(response.statusCode).toBe(200);
		expect(response.json().authenticated).toBe(true);
		expect(response.headers["x-user-id"]).toBeDefined();
	});

	it("should return 401 on /verify when no cookie is present", async () => {
		const req = { method: "GET" as const, url: "/verify" };
		const response = await server.inject(req);

		logInteraction("Verify No Cookie", req, response);
		expect(response.statusCode).toBe(401);
		expect(response.json()).toEqual({ error: "No active session." });
	});

	it("should delete the expired session row and return 401 on /verify", async () => {
		// Clean slate so we can assert an exact row count of 0 afterwards
		await server.db.execute(sql`DELETE FROM sessions WHERE user_id = (SELECT id FROM users WHERE email = 'bob@example.com')`);

		const cookie = await loginBob();

		// Manually expire the session
		await server.db.execute(sql`
			UPDATE sessions
			SET expires_at = NOW() - interval '1 day'
			WHERE user_id = (SELECT id FROM users WHERE email = 'bob@example.com')
		`);

		const req = { method: "GET" as const, url: "/verify", headers: { cookie } };
		const response = await server.inject(req);

		logInteraction("Verify Expired Session", req, response);
		expect(response.statusCode).toBe(401);

		// The row must have been deleted, not just skipped
		const result = await server.db.execute(sql`
			SELECT * FROM sessions WHERE user_id = (SELECT id FROM users WHERE email = 'bob@example.com')
		`);
		expect(result.rows.length).toBe(0);
	});

	it("should delete the session row and clear the cookie on /logout", async () => {
		const cookie = await loginBob();

		const req = {
			method: "POST" as const,
			url: "/logout",
			headers: { host: "localhost:8080", cookie },
		};
		const response = await server.inject(req);

		logInteraction("Logout", req, response);
		expect(response.statusCode).toBe(200);

		// Cookie must be cleared (empty value + past expiry)
		const setCookie = response.headers["set-cookie"];
		const raw = Array.isArray(setCookie) ? setCookie[0] : (setCookie ?? "");
		expect(raw).toMatch(/session_id=;/);

		// Session row must be gone from DB
		const result = await server.db.execute(sql`
			SELECT * FROM sessions WHERE user_id = (SELECT id FROM users WHERE email = 'bob@example.com')
		`);
		expect(result.rows.length).toBe(0);
	});

	it("should return 401 on /verify after logout", async () => {
		const cookie = await loginBob();

		// Logout first
		await server.inject({
			method: "POST",
			url: "/logout",
			headers: { host: "localhost:8080", cookie },
		});

		// The same cookie should now be invalid
		const req = { method: "GET" as const, url: "/verify", headers: { cookie } };
		const response = await server.inject(req);

		logInteraction("Verify After Logout", req, response);
		expect(response.statusCode).toBe(401);
	});

	it("should return 409, not 500, when the same user registers concurrently", async () => {
		const register = () =>
			server.inject({
				method: "POST",
				url: "/register",
				payload: { loginName: "testuser_race", email: "race@example.com", password: "securePassword123!" },
			});
		const statuses = (await Promise.all([register(), register(), register()])).map((r) => r.statusCode);

		expect(statuses.filter((s) => s === 200)).toHaveLength(1);
		expect(statuses.filter((s) => s === 409)).toHaveLength(2);
	});

	// Cleanup: Delete the test user so the test is repeatable
	afterAll(async () => {
		// We can access the db directly via the decorator we made!
		await server.db.execute(sql`DELETE FROM users WHERE email IN ('bob@example.com', 'race@example.com')`);
		await server.close(); // Close the pool after cleanup
	});
});
