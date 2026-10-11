use axum::extract::ws::{CloseFrame, Message, close_code};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::Instant;

use crate::db;
use crate::state::AppState;
use crate::title;

/// How long clients get to finish the close handshake before the remaining
/// rooms are saved directly. Kept well under Docker's 10 s stop timeout.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolves on SIGTERM (`docker stop`) or Ctrl+C.
pub async fn signal() {
	let ctrl_c = tokio::signal::ctrl_c();
	let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
		.expect("install SIGTERM handler");
	tokio::select! {
		_ = ctrl_c => {},
		_ = sigterm.recv() => {},
	}
}

/// Closes every connection with 1012 (service restart), so y-websocket reconnects to
/// the next instance, and makes sure every open document is saved before exit.
///
/// Each closed connection goes through the normal cleanup in ws_handler, which saves
/// the room when its last client leaves. Rooms still open at the deadline (clients
/// that never answered the close frame) are saved here.
pub async fn drain(state: &AppState) {
	state.shutting_down.store(true, Ordering::Release);
	let rooms: Vec<_> = state.rooms.iter().map(|r| r.value().clone()).collect();
	let clients: usize = rooms.iter().map(|r| r.clients.len()).sum();
	tracing::info!(
		"Shutting down: closing {} connections in {} rooms",
		clients,
		rooms.len()
	);

	for room in &rooms {
		for client in room.clients.iter() {
			let close = Message::Close(Some(CloseFrame {
				code: close_code::RESTART,
				reason: "server restarting".into(),
			}));
			// a full queue means the client is not reading; drop it without the handshake
			if client.tx.try_send(close).is_err() {
				client.evict.notify_one();
			}
		}
	}

	let deadline = Instant::now() + DRAIN_TIMEOUT;
	while !state.rooms.is_empty() && Instant::now() < deadline {
		tokio::time::sleep(Duration::from_millis(50)).await;
	}

	let left: Vec<_> = state
		.rooms
		.iter()
		.map(|r| (*r.key(), r.value().clone()))
		.collect();
	for (note_id, room) in &left {
		let doc = room.doc.read().await;
		let _ = db::save_note(&state.pool, *note_id, &doc, "shutdown").await;
		drop(doc);
		title::sync_title(state, room, *note_id).await;
	}
	tracing::info!(
		"Shutdown complete: {} rooms closed by their clients, {} saved at the deadline",
		rooms.len().saturating_sub(left.len()),
		left.len()
	);
}
