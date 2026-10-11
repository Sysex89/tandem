use axum::{
	extract::{
		Path, State,
		ws::{Message, WebSocket, WebSocketUpgrade},
	},
	http::{HeaderMap, StatusCode},
	response::{IntoResponse, Response},
};
use futures_util::{SinkExt, stream::StreamExt};
use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Notify, mpsc};
use uuid::Uuid;

use crate::db;
use crate::metrics;
use crate::models::Access;
use crate::state::{AppState, Client, DocumentRoom};
use crate::sync;
use crate::title;

/// Messages queued for one client before it counts as too slow and is disconnected.
/// The client reconnects and resyncs the full document, which is cheaper than
/// buffering without limit for a socket that is not being read.
const CLIENT_QUEUE: usize = 2048;

// global counter to assign unique IDs to clients (for awareness tracking)
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

// ws route handler
pub async fn ws_route(
	ws: WebSocketUpgrade,
	Path(note_id): Path<Uuid>,
	State(state): State<Arc<AppState>>,
	headers: HeaderMap,
) -> Response {
	// Draining for shutdown: the client retries and lands on the next instance
	if state.shutting_down.load(Ordering::Acquire) {
		return StatusCode::SERVICE_UNAVAILABLE.into_response();
	}

	// Extract the verified User ID from Nginx headers
	let user_id = match headers
		.get("X-User-Id")
		.and_then(|v| v.to_str().ok())
		.and_then(|v| Uuid::parse_str(v).ok())
	{
		Some(id) => id,
		None => {
			metrics::get()
				.connections_total
				.with_label_values(&["unauthorized"])
				.inc();
			return StatusCode::UNAUTHORIZED.into_response();
		}
	};

	// Check ownership or share access (and whether the user may edit)
	let Some(access) = db::access_level(&state.pool, note_id, user_id).await else {
		metrics::get()
			.connections_total
			.with_label_values(&["forbidden"])
			.inc();
		return StatusCode::FORBIDDEN.into_response();
	};
	metrics::get()
		.connections_total
		.with_label_values(&["accepted"])
		.inc();

	// Upgrade to WebSocket
	ws.on_upgrade(move |socket| handle_socket(socket, note_id, state, user_id, access))
}

async fn handle_socket(
	socket: WebSocket,
	note_id: Uuid,
	state: Arc<AppState>,
	user_id: Uuid,
	access: Access,
) {
	// Find existing room, or load from DB and atomically insert via the entry API
	// to prevent a race where two connections both create a room for the same note.
	let room = if let Some(existing_room) = state.rooms.get(&note_id) {
		existing_room.clone()
	} else {
		let doc = match db::load_note(&state.pool, note_id).await {
			Ok(doc) => doc,
			Err(_) => {
				tracing::error!("Failed to load document {}", note_id);
				return;
			}
		};

		match state.rooms.entry(note_id) {
			dashmap::mapref::entry::Entry::Occupied(e) => e.get().clone(),
			dashmap::mapref::entry::Entry::Vacant(e) => {
				tracing::info!("Loaded note {} from database", note_id);
				let new_room = Arc::new(DocumentRoom::new(doc));
				e.insert(new_room.clone());
				metrics::get().rooms_active.set(state.rooms.len() as i64);

				let state_bg = state.clone();
				let room_bg = new_room.clone();
				tokio::spawn(async move {
					loop {
						tokio::time::sleep(std::time::Duration::from_secs(5)).await;

						if !state_bg.rooms.contains_key(&note_id) {
							tracing::debug!(
								"Background task for note {} exiting (room no longer exists)",
								note_id
							);
							break;
						}

						if room_bg.dirty.load(Ordering::Acquire) {
							tracing::debug!("Background save for note {}", note_id);
							let doc_lock = room_bg.doc.read().await;
							if db::save_note(&state_bg.pool, note_id, &doc_lock, "periodic")
								.await
								.is_ok()
							{
								room_bg.dirty.store(false, Ordering::Release);
							}
							drop(doc_lock);
							title::sync_title(&state_bg, &room_bg, note_id).await;
						}
					}
				});

				new_room
			}
		}
	};

	// register client
	let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
	tracing::debug!("Registering client {} for note {}", client_id, note_id);

	// channel: tx to receive messages from other clients, rx to send messages to this client. Pipe tx -> rx.
	let (tx, mut rx) = mpsc::channel::<Message>(CLIENT_QUEUE);
	let evict = Arc::new(Notify::new());
	room.clients.insert(
		client_id,
		Client {
			tx,
			evict: evict.clone(),
		},
	);

	// Shared with the receive task so the gauge follows mid-session role changes.
	let can_write = Arc::new(AtomicBool::new(access.can_write()));
	metrics::get()
		.clients_connected
		.with_label_values(&[metrics::access_label(access.can_write())])
		.inc();

	// and another channel: sender to send messages to others, receiver to receive messages from this client
	let (mut sender, mut receiver) = socket.split();

	// send state vector and awareness info to the new client
	let init_sync: Vec<u8> = sync::generate_initial_sync(&room.doc).await;
	let _ = sender.send(Message::Binary(init_sync.into())).await;

	let init_awareness: Vec<u8> = sync::generate_initial_awareness(&room.awareness).await;
	let _ = sender.send(Message::Binary(init_awareness.into())).await;

	// everything that ends up at rx (from other clients) should be sent to the client via sender(ws).
	let mut send_task = tokio::spawn(async move {
		while let Some(msg) = rx.recv().await {
			if sender.send(msg).await.is_err() {
				break;
			}
		}
	});

	// main loop for receiving messages from the client.
	// Also re-checks access every 15s so that a revoked guest is evicted
	// without needing any cross-service signal from the notes service.
	let room_clone = room.clone();
	let state_recv = state.clone();
	let can_write_recv = can_write.clone();
	let mut recv_task = tokio::spawn(async move {
		let mut access = access;
		let mut access_check = tokio::time::interval(std::time::Duration::from_secs(15));
		// the first tick fires immediately; skip it (we already checked on upgrade)
		access_check.tick().await;

		loop {
			tokio::select! {
				maybe_msg = receiver.next() => {
					let Some(Ok(msg)) = maybe_msg else { break };
					if let Message::Binary(bytes) = msg {
						// give raw bytes to the sync logic. If we get a response (OK + non empty), we broadcast it.
						match sync::process_binary_message(&bytes, &room_clone.doc, &room_clone.awareness, &room_clone.dirty, access).await {
							Ok(response_bytes) => {
								if !response_bytes.is_empty() {
									let broadcast_msg = Message::Binary(response_bytes.into());

									let mut too_slow = Vec::new();
									for client in room_clone.clients.iter() {
										if *client.key() != client_id
											&& let Err(mpsc::error::TrySendError::Full(_)) = client.value().tx.try_send(broadcast_msg.clone())
										{
											too_slow.push(*client.key());
										}
									}
									// removed after iterating: removing while holding a DashMap shard would deadlock
									for slow in too_slow {
										if let Some((_, client)) = room_clone.clients.remove(&slow) {
											metrics::get().clients_evicted_total.inc();
											tracing::warn!("Client {} on note {} fell {} messages behind, disconnecting", slow, note_id, CLIENT_QUEUE);
											client.evict.notify_one();
										}
									}
								}
							}
							Err(e) => {
								tracing::error!("Error processing binary message from client {}: {}", client_id, e);
							}
						}
					}
				}
				_ = access_check.tick() => {
					match db::access_level(&state_recv.pool, note_id, user_id).await {
						// picks up role changes too, e.g. Edit downgraded to View
						Some(level) => {
							if level != access {
								let gauge = &metrics::get().clients_connected;
								gauge.with_label_values(&[metrics::access_label(access.can_write())]).dec();
								gauge.with_label_values(&[metrics::access_label(level.can_write())]).inc();
								can_write_recv.store(level.can_write(), Ordering::Relaxed);
							}
							access = level;
						}
						None => {
							metrics::get().access_revoked_total.inc();
							tracing::info!("Access revoked for user {} on note {}, closing client {}", user_id, note_id, client_id);
							break;
						}
					}
				}
			}
		}
	});

	// wait until discconect or error, then cleanup + save
	tokio::select! {
		_ = (&mut send_task) => tracing::debug!("Send task for client {} finished", client_id),
		_ = (&mut recv_task) => tracing::debug!("Receive task for client {} finished", client_id),
		_ = evict.notified() => tracing::debug!("Client {} evicted", client_id),
	};
	// dropping both halves of the socket closes the connection
	send_task.abort();
	recv_task.abort();

	room.clients.remove(&client_id);
	metrics::get()
		.clients_connected
		.with_label_values(&[metrics::access_label(can_write.load(Ordering::Relaxed))])
		.dec();
	tracing::info!("Client {} disconnected from note {}", client_id, note_id);

	if room.clients.is_empty() {
		tracing::info!("Last user left room {}. Saving to DB...", note_id);

		let trigger = if state.shutting_down.load(Ordering::Acquire) {
			"shutdown"
		} else {
			"last_client"
		};
		let doc_lock = room.doc.read().await;
		if db::save_note(&state.pool, note_id, &doc_lock, trigger)
			.await
			.is_err()
		{
			tracing::error!("Failed to save note {} to DB upon closing.", note_id);
		}
		drop(doc_lock);
		title::sync_title(&state, &room, note_id).await;

		state.rooms.remove(&note_id);
		metrics::get().rooms_active.set(state.rooms.len() as i64);
	}
}
