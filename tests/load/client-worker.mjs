// A slice of the simulated collaborators, run in its own thread so the load
// generator does not become the bottleneck. Each client is a real Yjs document
// synced through y-websocket, like the browser editor, and types into its own
// paragraph of the ProseMirror fragment while broadcasting its cursor.
import { monitorEventLoopDelay, performance } from "node:perf_hooks";
import { parentPort, workerData } from "node:worker_threads";
import WebSocket from "ws";
import { WebsocketProvider } from "y-websocket";
import * as Y from "yjs";

process.setMaxListeners(0); // y-websocket adds an exit listener per provider

const { wsUrl, noteId, clients, clientIdBase, totalClients, maxClock, rate, cursors, stalled } = workerData;

// Send timestamps shared by all workers: slot [writer * maxClock + clock].
// A receiver decodes the struct id of each update and looks up when it was typed.
const sentAt = new Float64Array(workerData.sentAt);
const now = () => performance.timeOrigin + performance.now();

const WORDS = "the quick brown fox jumps over the lazy dog ".split("");
const loop = monitorEventLoopDelay({ resolution: 10 });

let measuring = false;
const latencies = [];
const state = clients.map(({ index, cookie, writer }) => connect(index, cookie, writer));

function connect(index, cookie, writer) {
	class CookieWebSocket extends WebSocket {
		constructor(url, protocols) {
			super(url, protocols, { headers: { cookie }, rejectUnauthorized: false });
		}
	}

	const doc = new Y.Doc();
	doc.clientID = clientIdBase + index; // lets receivers map an update back to its writer
	const started = now();
	const provider = new WebsocketProvider(wsUrl, noteId, doc, {
		WebSocketPolyfill: CookieWebSocket,
		disableBc: true, // otherwise clients in the same thread sync without the server
		resyncInterval: -1,
	});
	const client = { index, writer, doc, provider, text: null, syncMs: null, disconnects: 0, sent: 0, timer: null };

	provider.on("status", ({ status }) => {
		if (status === "disconnected" && client.syncMs !== null) client.disconnects++;
	});
	provider.once("sync", () => {
		client.syncMs = now() - started;
		const paragraph = new Y.XmlElement("paragraph");
		client.text = new Y.XmlText();
		paragraph.insert(0, [client.text]);
		doc.getXmlFragment("default").push([paragraph]);
		provider.awareness.setLocalStateField("user", { name: `load ${index}`, color: "#3b82f6" });
		// A stalled client stops reading its socket, like a frozen tab or a hostile client:
		// the server's queue for it fills up and it should be disconnected, not buffered forever.
		if (stalled) provider.ws._socket.pause();
		if (state.every((c) => c.syncMs !== null)) parentPort.postMessage({ type: "ready" });
	});
	doc.on("update", (update, origin) => {
		if (!measuring || origin !== provider) return;
		const received = now();
		for (const { id } of Y.decodeUpdate(update).structs) {
			const writer = id.client - clientIdBase;
			if (writer < 0 || writer >= totalClients || id.clock >= maxClock) continue;
			const sent = sentAt[writer * maxClock + id.clock];
			if (sent) {
				latencies.push(received - sent);
				break;
			}
		}
	});
	return client;
}

function type(client) {
	const { doc, text, provider } = client;
	const clock = Y.getState(doc.store, doc.clientID);
	if (clock < maxClock) sentAt[client.index * maxClock + clock] = now();
	text.insert(text.length, WORDS[client.sent++ % WORDS.length]); // sent synchronously by the provider
	if (cursors) {
		const pos = Y.relativePositionToJSON(Y.createRelativePositionFromTypeIndex(text, text.length));
		provider.awareness.setLocalStateField("cursor", { anchor: pos, head: pos });
	}
}

parentPort.on("message", async (msg) => {
	if (msg.type === "start") {
		loop.enable();
		measuring = true;
		for (const client of state.filter((c) => c.writer)) {
			// spread the first keystrokes so the clients do not type in lockstep
			setTimeout(() => {
				client.timer = setInterval(() => type(client), 1000 / rate);
			}, Math.random() * (1000 / rate));
		}
	} else if (msg.type === "stop") {
		for (const client of state) clearInterval(client.timer);
		await new Promise((r) => setTimeout(r, msg.drainMs)); // let in-flight updates arrive
		measuring = false;
		loop.disable();
		const samples = Float64Array.from(latencies);
		parentPort.postMessage(
			{
				type: "result",
				samples,
				syncMs: state.map((c) => c.syncMs),
				sent: state.reduce((n, c) => n + c.sent, 0),
				disconnects: state.reduce((n, c) => n + c.disconnects, 0),
				loopP99Ms: loop.percentile(99) / 1e6,
			},
			[samples.buffer],
		);
	} else if (msg.type === "close") {
		for (const { provider, doc } of state) {
			provider.destroy();
			doc.destroy();
		}
		setTimeout(() => process.exit(0), 200);
	}
});
