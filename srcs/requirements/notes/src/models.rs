use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- OUTPUT MODELS (The objects you receive from API) ---

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
	pub id: Uuid,
	pub title: String,
	pub owner_id: Uuid,
	pub owner_url: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

// Role Enum
#[derive(Serialize, Deserialize, sqlx::Type, Debug)]
#[sqlx(type_name = "access_type", rename_all = "PascalCase")]
pub enum Role {
	View,
	Edit,
	Owner,
}

// Returned when a share is created
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Share {
	pub id: Uuid, // This is the share_id used for access and revoke
	pub note_id: Uuid,
	pub guest_id: Option<Uuid>, // null = public link
	pub access_role: Role,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

// Returned for owner: list of all shares on a note or across all notes
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ManagedShareItem {
	pub share_id: Uuid, // OWNER: use for /collab/access/{share_id} and /collab/revoke/{share_id}
	pub note_id: Uuid,
	pub note_title: String,
	pub guest_id: Option<Uuid>,
	pub access_role: Role,
	pub created_at: DateTime<Utc>,
}

// Returned for guest: list of notes shared with me
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ReceivedShareItem {
	pub share_id: Uuid, // GUEST: use for /collab/access/{share_id}
	pub note_id: Uuid,
	pub note_title: String,
	pub access_role: Role,
	pub owner_id: Uuid, // shows who shared the note with the person
	pub created_at: DateTime<Utc>,
}

// --- INPUT PAYLOADS (The objects you send to the api) ---

// Payload for: PUT /api/notes/{id}
#[derive(Deserialize)]
pub struct EditTitlePayload {
	pub title: String,
}

// Payload for: POST /api/notes/collab/
#[derive(Deserialize)]
pub struct ShareNotePayload {
	pub note_id: Uuid,
	pub guest_id: Option<Uuid>, // null for public link
	pub role: Role,             // "View" or "Edit"
}

#[derive(Deserialize)]
pub struct CreateNote {
	pub title: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct NoteExport {
	pub id: Uuid,
	pub title: String,
	pub owner_id: Option<Uuid>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub state_vector: Option<Vec<u8>>,
}
