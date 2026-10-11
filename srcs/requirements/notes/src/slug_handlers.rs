use crate::AppState;
use crate::handlers::get_user_id;
use crate::models::Note;
use axum::{
	Json,
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
};

pub async fn get_note_by_slug(
	State(state): State<AppState>,
	Path(slug): Path<String>,
	headers: HeaderMap,
) -> Result<Json<Note>, StatusCode> {
	let user_id = get_user_id(&headers).map_err(|_| StatusCode::UNAUTHORIZED)?;

	tracing::debug!("Searching for note by slug '{}' for user {}", slug, user_id);

	// First principles: Look for the slug in the owner_url column
	let note = sqlx::query_as::<_, Note>(
		"SELECT id, title, owner_id, owner_url, created_at, updated_at 
         FROM notes 
         WHERE owner_url = $1 AND owner_id = $2",
	)
	.bind(&slug)
	.bind(user_id)
	.fetch_optional(&state.db_pool)
	.await
	.map_err(|e| {
		tracing::error!("Failed to lookup note by slug {}: {}", slug, e);
		StatusCode::INTERNAL_SERVER_ERROR
	})?;

	match note {
		Some(note) => Ok(Json(note)),
		None => Err(StatusCode::NOT_FOUND),
	}
}
