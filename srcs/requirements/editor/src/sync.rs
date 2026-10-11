use crate::metrics;
use crate::models::Access;
use std::sync::atomic::{AtomicBool, Ordering};
use y_sync::awareness::Awareness;
use y_sync::sync::{Message, SyncMessage};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::{Encode, Encoder, EncoderV1};
use yrs::{Doc, ReadTxn, Transact, Update};

// the brain of the CRDT logic
pub async fn process_binary_message(
	msg_bytes: &[u8],
	doc: &tokio::sync::RwLock<Doc>,
	awareness: &tokio::sync::RwLock<Awareness>,
	dirty: &AtomicBool,
	access: Access,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
	// get raw message and tranform it into our internal Message struct (which can be a Sync message or a Awareness message)
	let msg = Message::decode_v1(msg_bytes).inspect_err(|_| {
		metrics::get()
			.messages_total
			.with_label_values(&["invalid"])
			.inc();
	})?;
	let kind = match &msg {
		Message::Sync(SyncMessage::SyncStep1(_)) => "sync_step1",
		Message::Sync(SyncMessage::SyncStep2(_)) => "sync_step2",
		Message::Sync(SyncMessage::Update(_)) => "update",
		Message::Awareness(_) => "awareness",
		_ => "other",
	};
	metrics::get()
		.messages_total
		.with_label_values(&[kind])
		.inc();

	// we will write the response into this encoder, which we will then turn into bytes at the end. If we dont have to respond, it will just be empty
	let mut response_encoder = EncoderV1::new();

	// main logic: if msg is a sync message, we apply it to the doc. If its an awareness message, we apply it to the awareness. Maybe future auth logic.
	match msg {
		Message::Sync(sync_msg) => {
			let doc_lock = doc.write().await;
			let mut txn = doc_lock.transact_mut();

			// if sync: there are two types of messages: SyncStep1 (client asks "what is the current text?") and SyncStep2/Update (client says "I typed this, apply it to your RAM").
			match sync_msg {
				SyncMessage::SyncStep1(state_vector) => {
					let update = txn.encode_state_as_update_v1(&state_vector);
					Message::Sync(SyncMessage::SyncStep2(update)).encode(&mut response_encoder);
				}
				SyncMessage::SyncStep2(_) | SyncMessage::Update(_) if !access.can_write() => {
					// Read-only client: drop its edits, never apply or broadcast them.
					metrics::get().updates_dropped_total.inc();
					tracing::debug!("Dropped document update from read-only client");
				}
				SyncMessage::SyncStep2(update_data) | SyncMessage::Update(update_data) => {
					let update = Update::decode_v1(&update_data)?;
					txn.apply_update(update);
					dirty.store(true, Ordering::Release);
					// Re-broadcast the update to other clients
					Message::Sync(SyncMessage::Update(update_data)).encode(&mut response_encoder);
				}
			}
		}

		// if cursor message: we apply the update but do not lock the document, since its just cursor movement.
		Message::Awareness(update) => {
			let mut awareness_lock = awareness.write().await;
			awareness_lock.apply_update(update)?;
			if let Ok(new_update) = awareness_lock.update() {
				Message::Awareness(new_update).encode(&mut response_encoder);
			}
		}

		_ => {}
	}

	Ok(response_encoder.to_vec())
}

// to helper func for ws_handler to generate the initial sync state or awareness state.
pub async fn generate_initial_sync(doc: &tokio::sync::RwLock<Doc>) -> Vec<u8> {
	let doc_lock = doc.read().await;
	let txn = doc_lock.transact();
	let state = txn.encode_state_as_update_v1(&yrs::StateVector::default());

	let mut encoder = EncoderV1::new();
	Message::Sync(SyncMessage::SyncStep2(state)).encode(&mut encoder);
	encoder.to_vec()
}

pub async fn generate_initial_awareness(awareness: &tokio::sync::RwLock<Awareness>) -> Vec<u8> {
	let awareness_lock = awareness.read().await;
	let update = awareness_lock.update().unwrap();

	let mut encoder = EncoderV1::new();
	Message::Awareness(update).encode(&mut encoder);
	encoder.to_vec()
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::sync::RwLock;
	use yrs::{GetString, StateVector, Text};

	/// A Yjs update, encoded as a y-sync message, that inserts `text` into "body".
	fn client_update(text: &str) -> Vec<u8> {
		let client = Doc::new();
		let body = client.get_or_insert_text("body");
		body.insert(&mut client.transact_mut(), 0, text);
		let update = client
			.transact()
			.encode_state_as_update_v1(&StateVector::default());
		Message::Sync(SyncMessage::Update(update)).encode_v1()
	}

	fn server() -> (RwLock<Doc>, RwLock<Awareness>, AtomicBool) {
		let doc = Doc::new();
		let awareness = Awareness::new(doc.clone());
		(
			RwLock::new(doc),
			RwLock::new(awareness),
			AtomicBool::new(false),
		)
	}

	async fn body_text(doc: &RwLock<Doc>) -> String {
		let doc = doc.read().await;
		let body = doc.get_or_insert_text("body");
		body.get_string(&doc.transact())
	}

	#[tokio::test]
	async fn read_write_client_updates_are_applied_and_broadcast() {
		let (doc, awareness, dirty) = server();
		let response = process_binary_message(
			&client_update("hello"),
			&doc,
			&awareness,
			&dirty,
			Access::ReadWrite,
		)
		.await
		.unwrap();

		assert_eq!(body_text(&doc).await, "hello");
		assert!(dirty.load(Ordering::Acquire));
		assert!(!response.is_empty(), "update should be re-broadcast");
	}

	#[tokio::test]
	async fn read_only_client_updates_are_dropped() {
		let dropped_before = metrics::get().updates_dropped_total.get();
		let (doc, awareness, dirty) = server();
		let response = process_binary_message(
			&client_update("vandalism"),
			&doc,
			&awareness,
			&dirty,
			Access::ReadOnly,
		)
		.await
		.unwrap();

		assert_eq!(body_text(&doc).await, "");
		assert!(!dirty.load(Ordering::Acquire), "nothing to save");
		assert!(response.is_empty(), "nothing to broadcast");
		assert!(
			metrics::get().updates_dropped_total.get() > dropped_before,
			"drop is counted"
		);
	}

	#[tokio::test]
	async fn read_only_client_can_still_sync_the_document() {
		let (doc, awareness, dirty) = server();
		process_binary_message(
			&client_update("shared text"),
			&doc,
			&awareness,
			&dirty,
			Access::ReadWrite,
		)
		.await
		.unwrap();

		// A viewer asks for everything it is missing (empty state vector).
		let step1 = Message::Sync(SyncMessage::SyncStep1(StateVector::default())).encode_v1();
		let response = process_binary_message(&step1, &doc, &awareness, &dirty, Access::ReadOnly)
			.await
			.unwrap();

		let Message::Sync(SyncMessage::SyncStep2(update)) = Message::decode_v1(&response).unwrap()
		else {
			panic!("expected SyncStep2");
		};
		let viewer = Doc::new();
		viewer
			.transact_mut()
			.apply_update(Update::decode_v1(&update).unwrap());
		let body = viewer.get_or_insert_text("body");
		assert_eq!(body.get_string(&viewer.transact()), "shared text");
	}
}
