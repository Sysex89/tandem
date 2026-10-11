use crate::AppState;
use crate::metrics;
use crate::models::{CreateNote, Note, NoteExport};
use axum::{
	Json,
	extract::{Path, State},
	http::HeaderMap,
	http::StatusCode,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
	message: String,
}

pub(crate) type ApiError = (StatusCode, Json<ErrorResponse>);

fn requested_locale(headers: &HeaderMap, default_locale: &str) -> String {
	headers
		.get("Accept-Language")
		.and_then(|v| v.to_str().ok())
		.and_then(|v| v.split(',').next())
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.map(ToString::to_string)
		.unwrap_or_else(|| default_locale.to_string())
}

fn error_response(state: &AppState, locale: &str, status: StatusCode, key: &str) -> ApiError {
	let msg = state.i18n.t(locale, key);
	(status, Json(ErrorResponse { message: msg }))
}

pub fn get_user_id(headers: &HeaderMap) -> Result<Uuid, ()> {
	headers
		.get("X-User-Id")
		.and_then(|v| v.to_str().ok())
		.and_then(|v| Uuid::parse_str(v).ok())
		.ok_or(())
}

pub async fn get_all_notes(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<Vec<Note>>, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::debug!("Fetching all notes for user {}", user_id);
	let notes = sqlx::query_as::<_, Note>("SELECT id, title, owner_id, owner_url, created_at, updated_at FROM notes WHERE owner_id = $1 ORDER BY created_at ASC")
		.bind(user_id)
		.fetch_all(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!("Failed to fetch notes: {}", e);
			error_response(&state, &locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
		})?;

	Ok(Json(notes))
}

pub async fn get_note(
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<Json<Note>, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::debug!("Fetching note {} for user {}", id, user_id);
	let note = sqlx::query_as::<_, Note>("SELECT id, title, owner_id, owner_url, created_at, updated_at FROM notes WHERE id = $1 AND owner_id = $2")
		.bind(id)
		.bind(user_id)
		.fetch_optional(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!("Failed to fetch note {}: {}", id, e);
			error_response(&state, &locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
		})?;

	match note {
		Some(note) => Ok(Json(note)),
		None => {
			tracing::warn!("Note {} not found for user {}", id, user_id);
			Err(error_response(
				&state,
				&locale,
				StatusCode::NOT_FOUND,
				"note-not-found",
			))
		}
	}
}

pub async fn post_note(
	State(state): State<AppState>,
	headers: HeaderMap,
	Json(payload): Json<CreateNote>,
) -> Result<Json<Note>, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::info!("Creating new note: {} for user {}", payload.title, user_id);

	let mut tx = state.db_pool.begin().await.map_err(|e| {
		tracing::error!("Failed to begin transaction: {}", e);
		error_response(
			&state,
			&locale,
			StatusCode::INTERNAL_SERVER_ERROR,
			"internal-error",
		)
	})?;

	// Generate a unique slug for the owner_url
	let slug = generate_unique_slug(&mut *tx, user_id, &payload.title, None)
		.await
		.map_err(|e| {
			tracing::error!("Failed to generate unique slug: {}", e);
			error_response(
				&state,
				&locale,
				StatusCode::INTERNAL_SERVER_ERROR,
				"internal-error",
			)
		})?;

	let note = sqlx::query_as::<_, Note>(
		"INSERT INTO notes (title, owner_id, owner_url) VALUES ($1, $2, $3) RETURNING id, title, owner_id, owner_url, created_at, updated_at",
	)
	.bind(&payload.title)
	.bind(user_id)
    .bind(&slug)
	.fetch_one(&mut *tx)
	.await
	.map_err(|e| {
		tracing::error!("Failed to insert note: {}", e);
		error_response(&state, &locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
	})?;

	let empty_state = vec![0u8, 0u8];

	sqlx::query("INSERT INTO note_states (note_id, state_vector) VALUES ($1, $2)")
		.bind(note.id)
		.bind(&empty_state)
		.execute(&mut *tx)
		.await
		.map_err(|e| {
			tracing::error!("Failed to insert note state: {}", e);
			error_response(
				&state,
				&locale,
				StatusCode::INTERNAL_SERVER_ERROR,
				"internal-error",
			)
		})?;

	tx.commit().await.map_err(|e| {
		tracing::error!("Failed to commit transaction: {}", e);
		error_response(
			&state,
			&locale,
			StatusCode::INTERNAL_SERVER_ERROR,
			"internal-error",
		)
	})?;

	metrics::inc_mutation("create");
	tracing::info!("Note {} created successfully", note.id);
	Ok(Json(note))
}

pub async fn del_note(
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::info!("Deleting note {} for user {}", id, user_id);
	let result = sqlx::query("DELETE FROM notes WHERE id = $1 AND owner_id = $2")
		.bind(id)
		.bind(user_id)
		.execute(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!("Failed to delete note {}: {}", id, e);
			error_response(
				&state,
				&locale,
				StatusCode::INTERNAL_SERVER_ERROR,
				"internal-error",
			)
		})?;

	if result.rows_affected() == 0 {
		tracing::warn!("Note {} not found for deletion (or not owned by user)", id);
		return Err(error_response(
			&state,
			&locale,
			StatusCode::NOT_FOUND,
			"note-not-found",
		));
	}

	metrics::inc_mutation("delete");
	tracing::info!("Note {} deleted successfully", id);
	Ok(StatusCode::NO_CONTENT)
}

pub async fn edit_title(
	State(state): State<AppState>,
	Path(id): Path<Uuid>,
	headers: HeaderMap,
	Json(payload): Json<CreateNote>,
) -> Result<Json<Note>, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::info!("Updating title for note {}: {}", id, payload.title);

	let mut tx = state.db_pool.begin().await.map_err(|e| {
		tracing::error!("Failed to begin transaction: {}", e);
		error_response(
			&state,
			&locale,
			StatusCode::INTERNAL_SERVER_ERROR,
			"internal-error",
		)
	})?;

	let slug = generate_unique_slug(&mut *tx, user_id, &payload.title, Some(id))
		.await
		.map_err(|e| {
			tracing::error!("Failed to generate unique slug for update: {}", e);
			error_response(
				&state,
				&locale,
				StatusCode::INTERNAL_SERVER_ERROR,
				"internal-error",
			)
		})?;

	let note = sqlx::query_as::<_, Note>(
		"UPDATE notes SET title = $1, owner_url = $2, updated_at = NOW() WHERE id = $3 AND owner_id = $4 RETURNING id, title, owner_id, owner_url, created_at, updated_at"
		)
		.bind(&payload.title)
        .bind(&slug)
		.bind(id)
		.bind(user_id)
		.fetch_optional(&mut *tx)
		.await
		.map_err(|e| {
			tracing::error!("Failed to update note {}: {}", id, e);
			error_response(&state, &locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
		})?;

	match note {
		Some(note) => {
			tx.commit().await.map_err(|e| {
				tracing::error!("Failed to commit transaction: {}", e);
				error_response(
					&state,
					&locale,
					StatusCode::INTERNAL_SERVER_ERROR,
					"internal-error",
				)
			})?;
			tracing::info!("Note {} updated successfully", id);
			Ok(Json(note))
		}
		None => {
			tracing::warn!("Note {} not found for update", id);
			Err(error_response(
				&state,
				&locale,
				StatusCode::NOT_FOUND,
				"note-not-found",
			))
		}
	}
}

pub async fn del_notes_by_owner(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::info!("Deleting all notes for user {}", user_id);

	sqlx::query("DELETE FROM notes WHERE owner_id = $1")
		.bind(user_id)
		.execute(&state.db_pool)
		.await
		.map_err(|e| {
			tracing::error!("Failed to delete notes for user {}: {}", user_id, e);
			error_response(
				&state,
				&locale,
				StatusCode::INTERNAL_SERVER_ERROR,
				"internal-error",
			)
		})?;

	Ok(StatusCode::NO_CONTENT)
}

pub async fn export_notes(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<Vec<NoteExport>>, ApiError> {
	let locale = requested_locale(&headers, state.i18n.default_locale());
	let user_id = get_user_id(&headers)
		.map_err(|_| error_response(&state, &locale, StatusCode::UNAUTHORIZED, "unauthorized"))?;
	tracing::info!("Exporting notes for user {}", user_id);

	let notes = sqlx::query_as::<_, NoteExport>(
		"SELECT n.id, n.title, n.owner_id, n.created_at, n.updated_at, ns.state_vector \
		 FROM notes n LEFT JOIN note_states ns ON n.id = ns.note_id WHERE n.owner_id = $1 ORDER BY n.created_at ASC",
	)
	.bind(user_id)
	.fetch_all(&state.db_pool)
	.await
	.map_err(|e| {
		tracing::error!("Failed to export notes for user {}: {}", user_id, e);
		error_response(&state, &locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
	})?;

	Ok(Json(notes))
}

pub fn slugify(title: &str) -> String {
	title
		.to_lowercase()
		.chars()
		.map(|c| if c.is_alphanumeric() { c } else { '-' })
		.collect::<String>()
		.split('-')
		.filter(|s| !s.is_empty())
		.collect::<Vec<_>>()
		.join("-")
}

pub async fn generate_unique_slug(
	tx: &mut sqlx::PgConnection,
	owner_id: Uuid,
	title: &str,
	exclude_id: Option<Uuid>,
) -> Result<String, sqlx::Error> {
	let base_slug = slugify(title);
	let mut slug = base_slug.clone();
	let mut count = 1;

	loop {
		let mut query = String::from(
			"SELECT EXISTS(SELECT 1 FROM notes WHERE owner_id = $1 AND owner_url = $2",
		);
		if exclude_id.is_some() {
			query.push_str(" AND id != $3");
		}
		query.push_str(")");

		let mut q = sqlx::query_scalar::<_, bool>(&query)
			.bind(owner_id)
			.bind(&slug);

		if let Some(id) = exclude_id {
			q = q.bind(id);
		}

		if !q.fetch_one(&mut *tx).await? {
			break;
		}

		slug = format!("{}-{}", base_slug, count);
		count += 1;
	}

	Ok(slug)
}
