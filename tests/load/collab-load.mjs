// Load test for real-time collaboration: N clients edit one note at the same time.
// For each step it reports how long clients take to join, how long a keystroke
// takes to reach every other client (end-to-end through nginx and the editor
// service), and the CPU / memory of the containers on the path.
//
// Requires a running stack (`make demo`); see DOCS/load-test.md.
//
//   cd tests/load && pnpm install
//   node collab-load.mjs [--clients 2,10,25,50,100] [--duration 30] [--rate 5] [--active 1]
//                        [--no-cursors] [--stalled 0] [--json out.json]
//
// --active   share of the clients that type; the rest only receive (e.g. 0.1 = one writer in ten)
// --stalled  extra clients that join and then stop reading their socket
import { execFile } from "node:child_process";
import { availableParallelism } from "node:os";
import { writeFileSync } from "node:fs";
import { parseArgs, promisify } from "node:util";
import { Worker } from "node:worker_threads";
import { Agent } from "undici";

const { values: opts } = parseArgs({
	options: {
		base: { type: "string", default: process.env.BASE_URL ?? "https://localhost:8443" },
		clients: { type: "string", default: "2,10,25,50,100" },
		duration: { type: "string", default: "30" }, // seconds of typing per step
		rate: { type: "string", default: "5" }, // keystrokes per second per client
		active: { type: "string", default: "1" },
		"no-cursors": { type: "boolean", default: false },
		stalled: { type: "string", default: "0" },
		workers: { type: "string", default: String(Math.max(1, Math.min(16, availableParallelism() - 4))) },
		json: { type: "string" },
	},
});
const steps = opts.clients.split(",").map(Number);
const durationS = Number(opts.duration);
const rate = Number(opts.rate);
const active = Number(opts.active);
const cursors = !opts["no-cursors"];
const stalledCount = Number(opts.stalled);
const workerCount = Number(opts.workers);
const wsUrl = `${opts.base.replace(/^http/, "ws")}/ws`;
const PASSWORD = "mycelium-load";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// --- HTTP setup through nginx (self-signed certificate in the demo) -----------

const dispatcher = new Agent({ connect: { rejectUnauthorized: false } });

async function api(path, { cookie, method = "GET", body } = {}) {
	const res = await fetch(`${opts.base}${path}`, {
		method,
		dispatcher,
		headers: { ...(body && { "content-type": "application/json" }), ...(cookie && { cookie }) },
		body: body && JSON.stringify(body),
	});
	return res;
}

async function signIn(name) {
	let res = await api("/api/auth/register", {
		method: "POST",
		body: { loginName: name, email: `${name}@load.local`, password: PASSWORD },
	});
	if (res.status === 409) {
		res = await api("/api/auth/login", { method: "POST", body: { identifier: name, password: PASSWORD } });
	}
	if (!res.ok) throw new Error(`sign in ${name}: ${res.status} ${await res.text()}`);
	const cookie = res.headers.getSetCookie().map((c) => c.split(";")[0]).join("; ");
	const me = await (await api("/api/auth/me", { cookie })).json();
	return { name, cookie, id: me.user.id };
}

async function inBatches(items, size, fn) {
	const out = [];
	for (let i = 0; i < items.length; i += size) out.push(...(await Promise.all(items.slice(i, i + size).map(fn))));
	return out;
}

// --- Measurements --------------------------------------------------------------

const percentile = (sorted, p) => (sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))] : NaN);

/** Current value of an editor counter, from Prometheus (scraped every 5 s). */
async function editorCounter(name) {
	try {
		const { stdout } = await promisify(execFile)("docker", [
			"exec", "prometheus", "wget", "-qO-", `http://localhost:9090/api/v1/query?query=sum(${name})`,
		]);
		return Number(JSON.parse(stdout).data.result[0]?.value[1] ?? NaN);
	} catch {
		return NaN;
	}
}

/** CPU % and memory of the containers a keystroke passes through. */
async function containerStats() {
	try {
		const { stdout } = await promisify(execFile)("docker", [
			"stats", "--no-stream", "--format", "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}", "editor", "nginx", "postgres",
		]);
		return Object.fromEntries(
			stdout.trim().split("\n").map((line) => {
				const [name, cpu, mem] = line.split("\t");
				return [name, { cpu: cpu.trim(), mem: mem.split("/")[0].trim() }];
			}),
		);
	} catch {
		return {}; // docker not reachable from here; the Grafana dashboard still has the server side
	}
}

async function runStep(n, owner, guests) {
	const created = await api("/api/notes", { cookie: owner.cookie, method: "POST", body: { title: `load test: ${n} clients` } });
	const noteId = (await created.json()).id;
	const evictedBefore = await editorCounter("editor_clients_evicted_total");
	await inBatches(guests.slice(0, n + stalledCount), 20, (g) =>
		api("/api/notes/collab/", { cookie: owner.cookie, method: "POST", body: { note_id: noteId, guest_id: g.id, role: "Edit" } }),
	);

	const maxClock = Math.ceil(rate * durationS * 1.2) + 64;
	const sentAt = new SharedArrayBuffer(n * maxClock * Float64Array.BYTES_PER_ELEMENT);
	const clientIdBase = 1_000_000 + Math.floor(Math.random() * 1_000_000_000);
	const writers = Math.max(1, Math.round(n * active));
	const threads = Math.min(workerCount, n);
	const spawn = (clients, stalled = false) =>
		new Worker(new URL("./client-worker.mjs", import.meta.url), {
			workerData: { wsUrl, noteId, clients, clientIdBase, totalClients: n, maxClock, rate, cursors, sentAt, stalled },
		});
	const workers = Array.from({ length: threads }, (_, w) => {
		const clients = [];
		for (let i = w; i < n; i += threads) clients.push({ index: i, cookie: guests[i].cookie, writer: i < writers });
		return spawn(clients);
	});
	if (stalledCount) {
		const clients = guests.slice(n, n + stalledCount).map((g, i) => ({ index: n + i, cookie: g.cookie, writer: false }));
		workers.push(spawn(clients, true));
	}
	const next = (type) => Promise.all(workers.map((w) => new Promise((resolve, reject) => {
		const onMessage = (msg) => msg.type === type && (w.off("message", onMessage), resolve(msg));
		w.on("message", onMessage);
		w.once("error", reject);
	})));

	await Promise.race([
		next("ready"),
		sleep(60_000).then(() => { throw new Error(`${n} clients did not all sync within 60 s`); }),
	]);
	await sleep(2000); // initial paragraphs and cursors settle
	const results = next("result");
	for (const w of workers) w.postMessage({ type: "start" });
	const typingStarted = Date.now();
	await sleep((durationS * 1000) / 2);
	const stats = await containerStats(); // mid-step, under full load
	await sleep(Math.max(0, typingStarted + durationS * 1000 - Date.now()));
	for (const w of workers) w.postMessage({ type: "stop", drainMs: 3000 });
	const parts = await results;
	for (const w of workers) w.postMessage({ type: "close" });

	const latencies = Float64Array.from(parts.flatMap((p) => Array.from(p.samples))).sort();
	const sync = parts.flatMap((p) => p.syncMs).sort((a, b) => a - b);
	const sent = parts.reduce((s, p) => s + p.sent, 0);
	const expected = sent * (n - 1); // every typing client's keystrokes reach the other n - 1
	await sleep(7000); // the last client leaving triggers the final save
	await api(`/api/notes/${noteId}`, { cookie: owner.cookie, method: "DELETE" });

	const evicted = (await editorCounter("editor_clients_evicted_total")) - evictedBefore;
	return {
		clients: n,
		writers,
		evicted,
		keystrokesPerS: Math.round(sent / durationS),
		deliveriesPerS: Math.round(latencies.length / durationS),
		delivered: expected ? latencies.length / expected : 1,
		latencyMs: { p50: percentile(latencies, 50), p95: percentile(latencies, 95), p99: percentile(latencies, 99), max: latencies.at(-1) },
		joinMs: { p50: percentile(sync, 50), p95: percentile(sync, 95) },
		disconnects: parts.reduce((s, p) => s + p.disconnects, 0),
		generatorLoopP99Ms: Math.max(...parts.map((p) => p.loopP99Ms)),
		containers: stats,
	};
}

// --- Main ----------------------------------------------------------------------

const maxClients = Math.max(...steps) + stalledCount;
console.log(`Signing in ${maxClients + 1} load users...`);
const owner = await signIn("load_owner");
const guests = await inBatches(
	Array.from({ length: maxClients }, (_, i) => `load_${String(i).padStart(3, "0")}`),
	8,
	signIn,
);

console.log(
	`${durationS}s per step, ${rate} keystrokes/s per typing client (${active * 100}% typing), cursors ${cursors ? "on" : "off"}, ` +
		`${stalledCount} stalled clients, ${workerCount} generator threads\n`,
);
const rows = [];
for (const n of steps) {
	process.stdout.write(`${n} clients... `);
	const r = await runStep(n, owner, guests);
	rows.push(r);
	const ms = (v) => (Number.isFinite(v) ? v.toFixed(1) : "-");
	console.log(
		`latency p50 ${ms(r.latencyMs.p50)} / p95 ${ms(r.latencyMs.p95)} / p99 ${ms(r.latencyMs.p99)} ms, ` +
			`delivered ${(r.delivered * 100).toFixed(1)}%, editor CPU ${r.containers.editor?.cpu ?? "?"}`,
	);
}

const fmt = (v) => (Number.isFinite(v) ? v.toFixed(0) : "-");
console.log(
	"\n| Clients | Typing | Keystrokes/s | Deliveries/s | Delivered | Latency p50 / p95 / p99 (ms) | Join p95 (ms) | editor CPU | editor mem | nginx CPU | Generator lag p99 (ms) | Evicted |",
);
console.log("| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |");
for (const r of rows) {
	const c = r.containers;
	console.log(
		`| ${r.clients} | ${r.writers} | ${r.keystrokesPerS} | ${r.deliveriesPerS} | ${(r.delivered * 100).toFixed(1)}% | ` +
			`${r.latencyMs.p50.toFixed(1)} / ${r.latencyMs.p95.toFixed(1)} / ${r.latencyMs.p99.toFixed(1)} | ${fmt(r.joinMs.p95)} | ` +
			`${c.editor?.cpu ?? "?"} | ${c.editor?.mem ?? "?"} | ${c.nginx?.cpu ?? "?"} | ${r.generatorLoopP99Ms.toFixed(0)} | ${fmt(r.evicted)} |`,
	);
}
if (opts.json) {
	const options = { durationS, rate, active, cursors, stalled: stalledCount, workerCount };
	writeFileSync(opts.json, JSON.stringify({ options, rows }, null, 2));
}
process.exit(0);
