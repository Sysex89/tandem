# Auth Service Integration Tests

## How to run the tests

From the repository root, with `make dev` running (the production image has no dev dependencies):

```bash
docker compose -f srcs/docker-compose-dev.yml exec auth pnpm test --run
```

Add `-t "<test name>"` to run a single test. CI runs the same suite against a throwaway Postgres.

---

These integration tests verify the authentication flow from the outside in, treating the Fastify server as a black box (mostly). We use `server.inject()` to simulate HTTP requests without the overhead of actual network calls.

## Test Suite: `Auth Routes (Integration)`

The tests run against the **running development database** (`docker-compose-dev.yml`). They apply pending migrations first, so a fresh database works too, and they clean up after themselves, so they can be repeated.

### 1. Registration (`POST /register`)
**Goal**: Verify a user can sign up.
- **Action**: Sends a POST request with `loginName`, `email`, and `password`.
- **Checks**:
    - **Status 200**.
    - **Body**: `{ success: true }`. Navigation is handled by the frontend, not the server.
    - **Cookie**: Checks that a `session_id` HttpOnly cookie is set (signed).

### 2. Duplicate Email Registration
**Goal**: Verify the email uniqueness constraint.
- **Action**: Tries to register `bob@example.com` again.
- **Checks**:
    - **Status 409**: Conflict.
    - **Error Message**: "Email already registered."

### 3. Duplicate LoginName Registration
**Goal**: Verify the username uniqueness constraint.
- **Action**: Tries to register a different email but the same `loginName`.
- **Checks**:
    - **Status 409**: Conflict.
    - **Error Message**: "Username already taken."

### 4. Login (`POST /login`)
**Goal**: Verify a user can return and log in.
- **Action**: Sends a POST request with the registered credentials.
- **Checks**:
    - **Status 200**.
    - **Body**: `{ success: true }`. Navigation is handled by the frontend, not the server.
    - **Cookie**: A new session cookie is issued.

### 5. Invalid Login — Wrong Password
**Goal**: Verify security against bad passwords.
- **Action**: Sends a POST request with the correct email but wrong password.
- **Checks**:
    - **Status 401**: Unauthorized.

### 6. Invalid Login — Non-existent User
**Goal**: Verify the same 401 response for unknown identifiers (no user enumeration).
- **Action**: Sends a POST request with an email that was never registered.
- **Checks**:
    - **Status 401**: Unauthorized.

### 7. Verify Valid Session (`GET /verify`)
**Goal**: Confirm the Nginx `auth_request` endpoint works for active sessions.
- **Action**: Logs in, uses the returned cookie on `GET /verify`.
- **Checks**:
    - **Status 200**.
    - `authenticated: true` in body.
    - `X-User-Id` response header is set.

### 8. Verify — No Cookie
**Goal**: Confirm unauthenticated requests are rejected.
- **Action**: Calls `GET /verify` with no cookie.
- **Checks**:
    - **Status 401**.

### 9. Verify — Expired Session (row deletion)
**Goal**: Confirm expired sessions are deleted from the DB, not just rejected.
- **Action**: Logs in, manually backdates `expires_at` in the DB, calls `GET /verify` with the old cookie.
- **Checks**:
    - **Status 401**.
    - The `sessions` row no longer exists in the DB.

### 10. Logout (`POST /logout`)
**Goal**: Verify full session revocation.
- **Action**: Logs in, then calls `POST /logout` with the session cookie.
- **Checks**:
    - **Status 200**.
    - `set-cookie` header clears the `session_id` cookie.
    - The `sessions` row no longer exists in the DB.

### 11. Verify After Logout
**Goal**: Confirm a revoked token cannot be reused.
- **Action**: Logs in, logs out, then calls `GET /verify` with the original cookie.
- **Checks**:
    - **Status 401**.

### 12. Concurrent Registration
**Goal**: Confirm a race on the unique constraints is reported as a conflict, not a server error.
- **Action**: Sends three identical `POST /register` requests at once.
- **Checks**:
    - Exactly one **200** and two **409**.

---

## Planned Tests

The following are not yet covered:

### Login Gaps
- User with no local account (GitHub-only user trying to use password login)

### User Management (no coverage yet)
- `GET /me` — returns profile with valid session; 401 without
- `PATCH /change-login` / `PATCH /change-email` — success + duplicate conflict (409)
- `POST /change-password` — verifies old password, updates hash
- `DELETE /delete-account` — removes user + session, cookie cleared

---

## Cleanup & Safety

**"Do we erase what we added?"**
**Yes.**

We use an `afterAll` hook to strictly remove the test data:

```typescript
afterAll(async () => {
    // Hard delete the test user based on the specific test email
    await server.db.execute(sql`DELETE FROM users WHERE email IN ('bob@example.com', 'race@example.com')`);
    await server.close(); 
});
```

We also run this **before** the tests start (`beforeAll`), just in case a previous test run crashed halfway through and left its users in the database.
