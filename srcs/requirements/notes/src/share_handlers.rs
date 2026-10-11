use crate::AppState;
use crate::handlers::get_user_id;
use crate::models::{ManagedShareItem, Note, ReceivedShareItem, Share, ShareNotePayload};
use axum::{
	Json,
	extract::{Path, State},
	http::HeaderMap,
	http::StatusCode,
};
use chrono::Utc;
use uuid::Uuid;

// Fetch all notes that are shared with the specific user
pub async fn shared_with_me(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<Vec<ReceivedShareItem>>, StatusCode> {
	let user_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::debug!("Fetching all notes shared with user {}", user_id);
	let shared_notes = sqlx::query_as::<_, ReceivedShareItem>(
		"SELECT s.id as share_id, n.id as note_id, n.title as note_title, s.access_role, n.owner_id, s.created_at
		 FROM notes n
		 INNER JOIN share s ON n.id = s.note_id
		 WHERE s.guest_id = $1
		 ORDER BY s.created_at DESC")
		.bind(user_id)
		.fetch_all(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!("Failed to fetch shared notes: {}", e);
			StatusCode::INTERNAL_SERVER_ERROR
		})?;

	Ok(Json(shared_notes))
}

// Fetch a single note that has been shared with the user
pub async fn open_share(
	State(state): State<AppState>,
	Path(share_id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<Json<Note>, StatusCode> {
	let user_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::debug!(
		"Fetching shared note via share {} for user {}",
		share_id,
		user_id
	);

	// A note is accessible if:
	// The user is the owner of the note.
	// The user is the designated guest in a share entry.
	// The share entry is public (guest_id is NULL).
	let note = sqlx::query_as::<_, Note>(
		"SELECT n.id, n.title, n.owner_id, n.owner_url, n.created_at, n.updated_at 
         FROM notes n 
         INNER JOIN share s ON n.id = s.note_id 
         WHERE s.id = $1 AND (n.owner_id = $2 OR s.guest_id = $2 OR s.guest_id IS NULL)",
	)
	.bind(share_id)
	.bind(user_id)
	.fetch_optional(&state.db_pool)
	.await
	.map_err(|e| {
		tracing::error!("Failed to fetch shared note via share {}: {}", share_id, e);
		StatusCode::INTERNAL_SERVER_ERROR
	})?;

	match note {
		Some(note) => Ok(Json(note)),
		None => Err(StatusCode::NOT_FOUND),
	}
}

// Fetch every single share linking to any note owned by this user
pub async fn my_shares(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<Vec<ManagedShareItem>>, StatusCode> {
	let owner_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::debug!("Fetching all shares managed by user {}", owner_id);

	let managed_shares = sqlx::query_as::<_, ManagedShareItem>(
        "SELECT s.id as share_id, n.id as note_id, n.title as note_title, s.guest_id, s.access_role, s.created_at
         FROM share s
         INNER JOIN notes n ON s.note_id = n.id
         WHERE n.owner_id = $1
         ORDER BY s.created_at DESC"
    )
    .bind(owner_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch managed shares: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

	Ok(Json(managed_shares))
}

// Fetch all shares associated with a specific note ID
pub async fn note_collaborators(
	State(state): State<AppState>,
	Path(note_id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<Json<Vec<ManagedShareItem>>, StatusCode> {
	let owner_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::debug!(
		"Fetching all shares for note {} by user {}",
		note_id,
		owner_id
	);

	let note_shares = sqlx::query_as::<_, ManagedShareItem>(
        "SELECT s.id as share_id, n.id as note_id, n.title as note_title, s.guest_id, s.access_role, s.created_at
         FROM share s
         INNER JOIN notes n ON s.note_id = n.id
         WHERE n.id = $1 AND n.owner_id = $2
         ORDER BY s.created_at DESC"
    )
    .bind(note_id)
    .bind(owner_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch shares for note {}: {}", note_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

	Ok(Json(note_shares))
}

pub async fn create_share(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(payload): Json<ShareNotePayload>,
) -> Result<Json<Share>, StatusCode> {
	let requesting_user_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::debug!(
		"Attempting to create a share for note {} by user {}",
		payload.note_id,
		requesting_user_id
	);

	let note_owner_id = sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM notes WHERE id = $1")
		.bind(payload.note_id)
		.fetch_optional(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!(
				"Failed to query note ownership for note {}: {}",
				payload.note_id,
				e
			);
			StatusCode::INTERNAL_SERVER_ERROR
		})?;

	let owner_id = match note_owner_id {
		Some(id) => id,
		None => return Err(StatusCode::NOT_FOUND),
	};

	if owner_id != requesting_user_id {
		return Err(StatusCode::FORBIDDEN);
	}

	if payload.guest_id == Some(requesting_user_id) {
		return Err(StatusCode::BAD_REQUEST);
	}

	let current_time = Utc::now();

	let new_share = sqlx::query_as::<_, Share>(
		"INSERT INTO share (note_id, guest_id, access_role, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, note_id, guest_id, access_role, created_at, updated_at",
	)
	.bind(payload.note_id)
	.bind(payload.guest_id)
	.bind(payload.role)
	.bind(current_time)
	.bind(current_time)
	.fetch_one(&state.db_pool)
	.await
	.map_err(|e| {
		if let Some(db_err) = e.as_database_error() {
			if db_err.is_unique_violation() {
				return StatusCode::CONFLICT;
			}
		}
		StatusCode::INTERNAL_SERVER_ERROR
	})?;

	tracing::info!(
		"Share {} created for note {}. Guest ID: {:?}, Role: {:?}",
		new_share.id,
		new_share.note_id,
		new_share.guest_id,
		new_share.access_role
	);

	Ok(Json(new_share))
}

pub async fn revoke_share(
	State(state): State<AppState>,
	Path(share_id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
	let requesting_user_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
	tracing::info!(
		"Attempting to revoke share {} by user {}",
		share_id,
		requesting_user_id
	);

	// Ensure the person attempting to revoke the share is the owner of the note
	let result = sqlx::query(
		"DELETE FROM share 
         USING notes 
         WHERE share.note_id = notes.id AND share.id = $1 AND notes.owner_id = $2",
	)
	.bind(share_id)
	.bind(requesting_user_id)
	.execute(&state.db_pool)
	.await
	.map_err(|e| {
		tracing::error!("Failed to revoke share {}: {}", share_id, e);
		StatusCode::INTERNAL_SERVER_ERROR
	})?;

	if result.rows_affected() == 0 {
		tracing::warn!(
			"Share {} not found or user {} is not the owner",
			share_id,
			requesting_user_id
		);
		return Err(StatusCode::NOT_FOUND);
	}

	tracing::info!("Share {} revoked successfully", share_id);
	Ok(StatusCode::NO_CONTENT)
}
