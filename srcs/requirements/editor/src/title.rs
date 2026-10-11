use crate::db;
use crate::state::{AppState, DocumentRoom};
use uuid::Uuid;
use yrs::{Doc, GetString, Transact};

/// The note title as stored in the shared Yjs document ("title" text).
pub fn title_of(doc: &Doc) -> String {
	let title = doc.get_or_insert_text("title");
	title.get_string(&doc.transact())
}

/// Pushes the shared title to the notes service if it changed since the last
/// push. The editor is the single writer of `notes.title`, so renames by any
/// collaborator reach the database, and notes whose shared title was never set
/// keep their stored title. Goes through the notes service so the slug is
/// regenerated too.
pub async fn sync_title(state: &AppState, room: &DocumentRoom, note_id: Uuid) {
	// write lock: get_or_insert_text opens a write transaction internally
	let current = title_of(&*room.doc.write().await);
	let mut saved = room.saved_title.lock().await;
	if *saved == current {
		return;
	}

	let Some(owner_id) = db::owner_of(&state.pool, note_id).await else {
		return;
	};
	let url = format!("{}/api/notes/{}", state.notes_url, note_id);
	let result = state
		.http
		.put(&url)
		.header("X-User-Id", owner_id.to_string())
		.json(&serde_json::json!({ "title": current }))
		.send()
		.await;

	match result {
		Ok(resp) if resp.status().is_success() => {
			crate::metrics::get()
				.title_syncs_total
				.with_label_values(&["ok"])
				.inc();
			tracing::debug!("Saved title for note {}", note_id);
			*saved = current;
		}
		Ok(resp) => {
			crate::metrics::get()
				.title_syncs_total
				.with_label_values(&["rejected"])
				.inc();
			tracing::warn!(
				"Notes service rejected title for note {}: {}",
				note_id,
				resp.status()
			);
		}
		Err(e) => {
			crate::metrics::get()
				.title_syncs_total
				.with_label_values(&["error"])
				.inc();
			tracing::warn!(
				"Could not reach notes service to save title for note {}: {}",
				note_id,
				e
			);
		}
	}
}
