# NGINX Service

nginx is the single entry point and the only place authentication is enforced for the backend services. The service image is built from `Dockerfile` (with a self-signed certificate for `localhost`); the hosted demo mounts `deploy/nginx.conf` over the config and runs behind Caddy instead.

## Files

| File | Contents |
| --- | --- |
| `conf/nginx.conf` | The servers below; copied to `/etc/nginx/conf.d/` |
| `conf/api-locations.conf` | `/api/notes` (behind `auth_request`) and `/api/auth`; included by the HTTPS and the internal server |
| `conf/app-locations.conf` | The 502 page, `/ws/` (behind `auth_request`, long timeouts for WebSockets) and `/` (frontend); included by the HTTPS server and by `deploy/nginx.conf` |
| `conf/custom_502.html` | Served as `/502.html` while a backend is down or restarting |

## Servers

| Listen | Published | Purpose |
| --- | --- | --- |
| 443 (TLS) | host port 8443 | The app: API, WebSocket, frontend, and Grafana at `/grafana/` |
| 80 | no | Redirect to HTTPS |
| 8081 | no | Plain HTTP for server-side rendering: the frontend calls `http://nginx:8081/api/...` so `auth_request` still applies, without trusting the self-signed certificate |
| 8088 | no | `stub_status` for nginx-exporter |

## The auth gate

Protected locations ask the auth service first and pass the verified identity on:

```nginx
location /api/notes {
	# 1. THE BOUNCER: a sub-request to auth's /verify; anything but 2xx stops here
	auth_request /api/auth/verify;

	# 2. THE EXTRACTION: read the identity from the sub-request's response headers
	# ('upstream_http_' is how nginx reads headers from internal sub-requests)
	auth_request_set $auth_user_id $upstream_http_x_user_id;

	# 3. THE INJECTOR: overwrite the header sent to the service. Even if a client
	# sends its own X-User-Id, nginx replaces it with the verified value.
	proxy_set_header X-User-Id $auth_user_id;

	proxy_pass http://$notes_upstream;
}
```

`/ws/` uses the same pattern for the editor service. The notes and editor services trust `X-User-Id` blindly, so any new protected route must be added here with `auth_request`.

`/api/auth/*` is proxied to `auth:3000` with the prefix stripped (`rewrite ^/api/auth/(.*) /$1 break;`), keeping `Host`, `X-Real-IP`, `X-Forwarded-For` and `X-Forwarded-Proto`.

Upstreams are written as variables (`set $notes_upstream notes:3003;`) together with `resolver 127.0.0.11`, Docker's DNS. The next section explains why.

## Resolver purpose

1. The Startup Crash (Static Mode)
If you use `proxy_pass http://auth:3000;` (no variables):

The Fact: Nginx tries to "verify its world" before it opens its doors.
The Event: If the auth container is not yet "Born" (started), Nginx says: "My world is broken! I cannot find 'auth' in the phonebook!" and it exits with an error.
The Cycle: If Docker has restart: always, it will indeed keep killing and restarting Nginx in a loop until auth finally appears. This is valid logic, but it is Brute Force.

2. The Runtime 502 (Static Mode)
If Nginx starts successfully (because auth was up), but then auth goes down later:

The Fact: Nginx does not crash.
The Problem: It keeps trying to send traffic to the Old IP it learned during startup. Since that IP is now a "ghost," you get a 502 Bad Gateway.
The Failure: Even if auth restarts and gets a New IP, Nginx will never know. You must manually restart Nginx to force it to look at the phonebook again.

3. The "Dependency Loop" Hazard
Relying on "Crash and Restart" is dangerous for Structural Stability.

Imagine a complex system:

Nginx depends on Auth.
Auth depends on Postgres.
Postgres takes 20 seconds to boot up.
Without the resolver, Nginx will crash-restart 10 times while waiting for Postgres to be ready for Auth. In a large system, this "Thundering Herd" of restarting containers can overwhelm your CPU and memory, sometimes preventing anything from ever successfully starting.

