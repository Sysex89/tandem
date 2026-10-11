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
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
	// get raw message and tranform it into our internal Message struct (which can be a Sync message or a Awareness message)
	let msg = Message::decode_v1(msg_bytes)?;

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
