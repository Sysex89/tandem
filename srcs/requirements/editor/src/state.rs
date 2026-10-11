use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc::UnboundedSender};
use uuid::Uuid;
use yrs::Doc;
use y_sync::awareness::Awareness;
use axum::extract::ws::Message;

//	Represents an active document in RAM
pub struct DocumentRoom {
	pub doc: Arc<RwLock<Doc>>,
	pub awareness: Arc<RwLock<Awareness>>,
	pub clients: DashMap<usize, UnboundedSender<Message>>,
	pub dirty: AtomicBool,
}

impl DocumentRoom {
	pub fn new(doc: Doc) -> Self {
		let awareness = Arc::new(RwLock::new(Awareness::new(doc.clone())));
		Self {
			doc: Arc::new(RwLock::new(doc)),
			awareness,
			clients: DashMap::new(),
			dirty: AtomicBool::new(false),
		}
	}
}

//	The global state that is passed to Axum in main.rs
pub struct AppState {
	pub pool: PgPool,
	pub rooms: DashMap<Uuid, Arc<DocumentRoom>>,
}

impl AppState {
	pub fn new(pool: PgPool) -> Self {
		Self {
			pool,
			rooms: DashMap::new(),
		}
	}
}