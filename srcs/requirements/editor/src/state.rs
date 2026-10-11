use axum::extract::ws::Message;
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{Mutex, RwLock, mpsc::UnboundedSender};
use uuid::Uuid;
use y_sync::awareness::Awareness;
use yrs::Doc;

//	Represents an active document in RAM
pub struct DocumentRoom {
	pub doc: Arc<RwLock<Doc>>,
	pub awareness: Arc<RwLock<Awareness>>,
	pub clients: DashMap<usize, UnboundedSender<Message>>,
	pub dirty: AtomicBool,
	// Last title written to the notes service (see title::sync_title)
	pub saved_title: Mutex<String>,
}

impl DocumentRoom {
	pub fn new(doc: Doc) -> Self {
		let saved_title = Mutex::new(crate::title::title_of(&doc));
		let awareness = Arc::new(RwLock::new(Awareness::new(doc.clone())));
		Self {
			doc: Arc::new(RwLock::new(doc)),
			awareness,
			clients: DashMap::new(),
			dirty: AtomicBool::new(false),
			saved_title,
		}
	}
}

//	The global state that is passed to Axum in main.rs
pub struct AppState {
	pub pool: PgPool,
	pub rooms: DashMap<Uuid, Arc<DocumentRoom>>,
	pub http: reqwest::Client,
	/// Base URL of the notes service on the internal network
	pub notes_url: String,
}

impl AppState {
	pub fn new(pool: PgPool) -> Self {
		Self {
			pool,
			rooms: DashMap::new(),
			http: reqwest::Client::new(),
			notes_url: std::env::var("NOTES_URL")
				.unwrap_or_else(|_| "http://notes:3003".to_string()),
		}
	}
}
