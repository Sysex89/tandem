use sqlx::FromRow;

/// What a connected user may do with a note's document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
	/// Receives the document and live updates, but its own edits are dropped.
	ReadOnly,
	ReadWrite,
}

impl Access {
	pub fn can_write(self) -> bool {
		self == Access::ReadWrite
	}
}

// I thought this will be more complex, but I think this is enough.
#[derive(FromRow)]
pub struct SavedNoteState {
	pub state_vector: Vec<u8>,
}
