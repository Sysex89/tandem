use crate::models::{Access, SavedNoteState};
use sqlx::PgPool;
use uuid::Uuid;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update, updates::decoder::Decode};

// loads state of a note from the database and applies it to a new yrs::Doc, which is then returned. This is called when a new client connects to load the current state of the document.
pub async fn load_note(pool: &PgPool, note_id: Uuid) -> Result<Doc, ()> {
	// get blob
	let result = sqlx::query_as::<_, SavedNoteState>(
		"SELECT state_vector FROM note_states WHERE note_id = $1",
	)
	.bind(note_id)
	.fetch_optional(pool)
	.await
	.map_err(|e| {
		tracing::error!("DB Error loading note {}: {}", note_id, e);
	})?;

	let doc = Doc::new();

	// apply blob to doc
	if let Some(saved_state) = result
		&& !saved_state.state_vector.is_empty()
	{
		let mut txn = doc.transact_mut();
		match Update::decode_v1(&saved_state.state_vector) {
			Ok(update) => {
				txn.apply_update(update);
				tracing::debug!("Successfully applied saved state to note {}", note_id);
			}
			Err(e) => {
				tracing::error!("CRDT Decode Error for {}: {}", note_id, e);
				return Err(());
			}
		}
	}

	Ok(doc)
}

pub async fn owner_of(pool: &PgPool, note_id: Uuid) -> Option<Uuid> {
	sqlx::query_scalar("SELECT owner_id FROM notes WHERE id = $1")
		.bind(note_id)
		.fetch_optional(pool)
		.await
		.map_err(|e| tracing::error!("DB Error fetching owner of note {}: {}", note_id, e))
		.ok()
		.flatten()
}

/// Owner, or a guest via a private share or public link. The most permissive
/// matching role wins; `None` means no access (or a database error).
pub async fn access_level(pool: &PgPool, note_id: Uuid, user_id: Uuid) -> Option<Access> {
	let roles: Vec<String> = sqlx::query_scalar(
		"SELECT 'Owner' FROM notes WHERE id = $1 AND owner_id = $2
		 UNION ALL
		 SELECT access_role::text FROM share WHERE note_id = $1 AND (guest_id = $2 OR guest_id IS NULL)",
	)
	.bind(note_id)
	.bind(user_id)
	.fetch_all(pool)
	.await
	.map_err(|e| tracing::error!("DB Error checking access to note {}: {}", note_id, e))
	.ok()?;

	if roles.iter().any(|r| r == "Owner" || r == "Edit") {
		Some(Access::ReadWrite)
	} else if roles.iter().any(|r| r == "View") {
		Some(Access::ReadOnly)
	} else {
		None
	}
}

pub async fn save_note(pool: &PgPool, note_id: Uuid, doc: &Doc) -> Result<(), ()> {
	// get state vector as blob
	let state_blob = {
		let txn = doc.transact();
		txn.encode_state_as_update_v1(&StateVector::default())
	};

	// save to db
	sqlx::query(
		"UPDATE note_states SET state_vector = $1, last_saved_at = NOW() WHERE note_id = $2",
	)
	.bind(&state_blob)
	.bind(note_id)
	.execute(pool)
	.await
	.map_err(|e| {
		tracing::error!("DB Error saving note {}: {}", note_id, e);
	})?;

	tracing::info!("Successfully saved note {}", note_id);
	Ok(())
}
