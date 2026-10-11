use sqlx::FromRow;

// I thought this will be more complex, but I think this is enough.
#[derive(FromRow)]
pub struct SavedNoteState {
	pub state_vector: Vec<u8>,
}
