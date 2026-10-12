# Learn the Auth Service Architecture 🧠

This document breaks down the **Why** and **How** of our Authentication service. It explains the specific tools we chose, how they fit together, and walks through the initialization flow from "First Principles."

---

## 1. The Building Blocks (External Dependencies)

Before we look at *our* code, we must understand the tools we are using to build the foundation. We don't reinvent the wheel; we assemble the best wheels available.

### **The Core: Fastify**
We choose **Fastify** over Express because of its minimal overhead and native support for asynchronous code. It essentially acts as a highly efficient router that passes requests through a series of "Plugins."

### **The Database Stack**
We use a two-layer approach for data persistence:
1.  **@fastify/postgres**: This is the *physical connection layer*. It manages the raw TCP connection pool to the PostgreSQL database. It handles the "plumbing."
2.  **Drizzle ORM**: This is the *logic layer*. It sits on top of the raw connection. We use it to write type-safe queries (e.g., `db.select().from(users)`).
    *   *Why?* It prevents SQL injection by default and ensures that if our TypeScript compiles, our SQL queries are likely correct.

### **The Security Stack**
1.  **@fastify/cookie**:
    *   **Purpose**: Stores the Session ID on the user's browser.
    *   **Mechanism**: We use *Signed Cookies*. The server uses a `SESSION_SECRET` to sign the cookie. If a user tries to modify their cookie (e.g., changing `user_id=1` to `user_id=admin`), the signature validation will fail, and the server will reject it.
2.  **@fastify/oauth2**:
    *   **Purpose**: Wrapper around the standard authorization code flow for GitHub. It handles the redirection to GitHub and the exchange of the temporary code for an access token.
3.  **Argon2**:
    *   **Purpose**: Hashing passwords.
    *   **Why specific?**: Older algorithms (MD5, SHA-256) are too fast, making them vulnerable to brute-force attacks. Argon2 is *memory-hard*, meaning it requires significant RAM to compute, making it prohibitively expensive for hackers to crack passwords using GPUs.

---

## 2. Our Internal Plugins (Modules)

In Fastify, "Everything is a Plugin." This philosophy means we structure our own application logic ([`src/routes/...`](./src/routes/)) exactly the same way we use external libraries.

### **How to Create a Plugin**

A plugin is simply an asynchronous function that receives the `server` instance as its first argument. This allows the plugin to register routes, add hooks, or even register other plugins, all while keeping the global scope clean.

```typescript
import { FastifyPluginAsync } from "fastify";

// 1. We export an async function
export const myPlugin: FastifyPluginAsync = async (server) => {
    
    // 2. We can use the server instance to register routes
    server.get("/hello", async (request, reply) => {
        return { hello: "world" };
    });

    // 3. We can access decorated properties (like our database)
    const users = await server.db.select().from(...);
};
```

### **Our Modules**

*   **[`src/routes/auth.ts`](./src/routes/auth.ts)**:
    *   Registers functionality for the *Login* flow.
    *   **POST /register** and **POST /login**: local accounts (hashing and verifying passwords with Argon2).
    *   **GET /login/github/callback**: The destination GitHub sends users to. It exchanges the code for a token, finds/creates the user in DB, and issues a session cookie.

*   **[`src/routes/sessions.ts`](./src/routes/sessions.ts)**:
    *   Registers functionality for *Verification*.
    *   **GET /verify**: Only returns 200 OK if the session cookie is valid.
    *   Used by Nginx (`auth_request`) to check if a request is allowed to pass through to protected services (like the Notes app).
    *   **POST /logout**: deletes the session row and clears the cookie.

*   **[`src/routes/user.ts`](./src/routes/user.ts)**:
    *   The profile: **GET /me**, user lookup for sharing (`/users`, `/resolve`), changing login name, email, picture and password, the GDPR export and account deletion (which also asks the notes service to delete the user's notes).

---

## 3. The Entry Point: [`src/index.ts`](./src/index.ts) (The Launcher)

This file has one job: **Interface with Reality**.

The code inside [`src/app.ts`](./src/app.ts) is pure logic—it doesn't know about ports, Docker, or secrets. `index.ts` is responsible for bridging that gap.

### **Step A: Loading Secrets**
We prioritize security. Secrets (like DB passwords or OAuth keys) should **not** be environment variables if possible, because environment variables can leak into logs.
We read from `/run/secrets/...`. This is the Docker Swarm/Compose standard for secure file mounting.

```typescript
if (existsSync(gitSecretPath)) {
    clientSecretGit = readFileSync(gitSecretPath, "utf8").trim();
}
```

### **Step B:Create Config Object for Fastify APP**
We create a configuration object (`AppConfig`) that holds the "Truth" of the current deployment (Database URL, Client IDs).

### **Step C: Binding**
Finally, `index.ts` tells the server to actually start listening on `0.0.0.0:3000`.

---

## 4. The Recipe: [`src/app.ts`](./src/app.ts) (The Factory)

This file exports `buildServer(config)`. It creates the application instance. This separation allows us to run integration tests by "building" a server without "listening" on a network port.

### **The Flow of `buildServer`**

1.  **Fastify Instance Creation**:
    We enable `trustProxy: true`. This is crucial because our service sits behind Nginx. Without this, the service would think every request comes from `127.0.0.1` (Nginx's IP) rather than the actual user's IP.

2.  **Database Connection**:
    We register `@fastify/postgres` first.
    *   *Hook*: We use `server.after()` to wait for the connection to be established. Only *then* do we initialize Drizzle and run migrations. This ensures we never try to query a database that isn't there yet.

3.  **Variable Injection (Decorators)**:
    We use `declare module "fastify"` to teach TypeScript about our custom properties.
    ```typescript
    server.decorate("db", db); // Now we can use server.db anywhere!
    ```

4.  **Plugin Registration**:
    *   **Cookies**: Registered before auth, because auth needs to set cookies.
    *   **OAuth2**: Configured here.
        *   The `callbackUri` comes from the `GITHUB_CALLBACK_URL` environment variable, and after login the user is redirected to `WEBSITE_URL`. Both are set per deployment (`https://localhost:8443` locally, the public domain for the hosted demo).

5.  **Routes**:
    Finally, we register our internal routes (`authRoutes`, `sessionRoutes`, `userManagementRoutes`).

---

## Summary

*   **[`src/index.ts`](./src/index.ts)** gathers the ingredients (Env vars, Secrets) and starts the fire (Port binding).
*   **[`src/app.ts`](./src/app.ts)** follows the recipe to mix the ingredients (Plugins, Database) into a cake (The Server Instance).
*   **[`src/routes/`](./src/routes/)** are the slices of cake served to the user.

*To see the specific routes available and visual diagrams of the authentication flow (SSR & Nginx), please consult the [Endpoints & Wiring Guide](./README.md#endpoints--wiring-guide) section of the main README.*

