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
	atomic::{AtomicUsize, Ordering},
};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db;
use crate::state::{AppState, DocumentRoom};
use crate::sync;

// global counter to assign unique IDs to clients (for awareness tracking)
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

// ws route handler
pub async fn ws_route(
	ws: WebSocketUpgrade,
	Path(note_id): Path<Uuid>,
	State(state): State<Arc<AppState>>,
	headers: HeaderMap,
) -> Response {
	// Extract the verified User ID from Nginx headers
	let user_id = match headers
		.get("X-User-Id")
		.and_then(|v| v.to_str().ok())
		.and_then(|v| Uuid::parse_str(v).ok())
	{
		Some(id) => id,
		None => return StatusCode::UNAUTHORIZED.into_response(),
	};

	// Check ownership or share access
	if !db::check_access(&state.pool, note_id, user_id).await {
		return StatusCode::FORBIDDEN.into_response();
	}

	// Upgrade to WebSocket
	ws.on_upgrade(move |socket| handle_socket(socket, note_id, state, user_id))
}

async fn handle_socket(socket: WebSocket, note_id: Uuid, state: Arc<AppState>, user_id: Uuid) {
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
							if db::save_note(&state_bg.pool, note_id, &doc_lock)
								.await
								.is_ok()
							{
								room_bg.dirty.store(false, Ordering::Release);
							}
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
	let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
	room.clients.insert(client_id, tx.clone());

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
	let mut recv_task = tokio::spawn(async move {
		let mut access_check = tokio::time::interval(std::time::Duration::from_secs(15));
		// the first tick fires immediately; skip it (we already checked on upgrade)
		access_check.tick().await;

		loop {
			tokio::select! {
				maybe_msg = receiver.next() => {
					let Some(Ok(msg)) = maybe_msg else { break };
					if let Message::Binary(bytes) = msg {
						// give raw bytes to the sync logic. If we get a response (OK + non empty), we broadcast it.
						match sync::process_binary_message(&bytes, &room_clone.doc, &room_clone.awareness, &room_clone.dirty).await {
							Ok(response_bytes) => {
								if !response_bytes.is_empty() {
									let broadcast_msg = Message::Binary(response_bytes.into());

									for client in room_clone.clients.iter() {
										if *client.key() != client_id {
											let _ = client.value().send(broadcast_msg.clone());
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
					if !db::check_access(&state_recv.pool, note_id, user_id).await {
						tracing::info!("Access revoked for user {} on note {}, closing client {}", user_id, note_id, client_id);
						break;
					}
				}
			}
		}
	});

	// wait until discconect or error, then cleanup + save
	tokio::select! {
		_ = (&mut send_task) => tracing::debug!("Send task for client {} finished", client_id),
		_ = (&mut recv_task) => tracing::debug!("Receive task for client {} finished", client_id),
	};

	room.clients.remove(&client_id);
	tracing::info!("Client {} disconnected from note {}", client_id, note_id);

	if room.clients.is_empty() {
		tracing::info!("Last user left room {}. Saving to DB...", note_id);

		let doc_lock = room.doc.read().await;
		if let Err(_) = db::save_note(&state.pool, note_id, &doc_lock).await {
			tracing::error!("Failed to save note {} to DB upon closing.", note_id);
		}

		state.rooms.remove(&note_id);
	}
}
