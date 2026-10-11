//! API tests: requests go through the real router into a throwaway Postgres database
//! (`#[sqlx::test]` creates one per test and applies `migrations/`).
//! Needs `DATABASE_URL` pointing at a server where the user may create databases;
//! `make test-notes` starts one.
//!
//! nginx is not in the loop: it guarantees that `X-User-Id` is the verified user,
//! and these tests check what the service does with it.

use axum::{
	Router,
	body::Body,
	http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{AppState, i18n::I18n, router};

fn app(pool: PgPool) -> Router {
	router(AppState {
		db_pool: pool,
		i18n: I18n::new(),
	})
}

/// A request under construction; `send()` returns the status and the JSON body (or Null).
struct Call<'a> {
	app: &'a Router,
	method: Method,
	uri: String,
	user: Option<String>,
	body: Option<Value>,
	lang: Option<&'a str>,
}

impl<'a> Call<'a> {
	fn by(mut self, user: Uuid) -> Self {
		self.user = Some(user.to_string());
		self
	}
	fn raw_user(mut self, header: &str) -> Self {
		self.user = Some(header.to_string());
		self
	}
	fn json(mut self, body: Value) -> Self {
		self.body = Some(body);
		self
	}
	fn lang(mut self, lang: &'a str) -> Self {
		self.lang = Some(lang);
		self
	}
	async fn send(self) -> (StatusCode, Value) {
		let mut req = Request::builder().method(self.method).uri(self.uri);
		if let Some(user) = self.user {
			req = req.header("X-User-Id", user);
		}
		if let Some(lang) = self.lang {
			req = req.header("Accept-Language", lang);
		}
		let req = match self.body {
			Some(body) => req
				.header("content-type", "application/json")
				.body(Body::from(body.to_string())),
			None => req.body(Body::empty()),
		}
		.unwrap();
		let res = self.app.clone().oneshot(req).await.unwrap();
		let status = res.status();
		let bytes = res.into_body().collect().await.unwrap().to_bytes();
		(
			status,
			serde_json::from_slice(&bytes).unwrap_or(Value::Null),
		)
	}
}

fn call(app: &Router, method: Method, uri: impl Into<String>) -> Call<'_> {
	Call {
		app,
		method,
		uri: uri.into(),
		user: None,
		body: None,
		lang: None,
	}
}

async fn create_note(app: &Router, owner: Uuid, title: &str) -> Value {
	let (status, note) = call(app, Method::POST, "/api/notes")
		.by(owner)
		.json(json!({ "title": title }))
		.send()
		.await;
	assert_eq!(status, StatusCode::OK, "create {title:?}: {note}");
	note
}

async fn rename(app: &Router, user: Uuid, note: &Value, title: &str) -> (StatusCode, Value) {
	call(
		app,
		Method::PUT,
		format!("/api/notes/{}", note["id"].as_str().unwrap()),
	)
	.by(user)
	.json(json!({ "title": title }))
	.send()
	.await
}

async fn share(
	app: &Router,
	owner: Uuid,
	note: &Value,
	guest: Option<Uuid>,
	role: &str,
) -> (StatusCode, Value) {
	call(app, Method::POST, "/api/notes/collab/")
		.by(owner)
		.json(json!({ "note_id": note["id"], "guest_id": guest, "role": role }))
		.send()
		.await
}

async fn list(app: &Router, user: Uuid, uri: &str) -> Vec<Value> {
	let (status, body) = call(app, Method::GET, uri).by(user).send().await;
	assert_eq!(status, StatusCode::OK, "GET {uri}: {body}");
	body.as_array().unwrap().clone()
}

async fn count(pool: &PgPool, sql: &str, note: &Value) -> i64 {
	let note_id = Uuid::parse_str(note["id"].as_str().unwrap()).unwrap();
	sqlx::query_scalar(sql)
		.bind(note_id)
		.fetch_one(pool)
		.await
		.unwrap()
}

fn user() -> Uuid {
	Uuid::new_v4()
}

// --- Identity ------------------------------------------------------------------

#[sqlx::test]
async fn requests_without_a_valid_user_id_are_rejected(pool: PgPool) {
	let app = app(pool);
	let id = Uuid::new_v4();
	let routes = [
		(Method::GET, "/api/notes".to_string()),
		(Method::POST, "/api/notes".to_string()),
		(Method::GET, format!("/api/notes/{id}")),
		(Method::PUT, format!("/api/notes/{id}")),
		(Method::DELETE, format!("/api/notes/{id}")),
		(Method::DELETE, "/api/notes/by-owner".to_string()),
		(Method::GET, "/api/notes/export".to_string()),
		(Method::GET, "/api/notes/u/some-slug".to_string()),
		(Method::GET, "/api/notes/collab/received".to_string()),
		(Method::GET, "/api/notes/collab/created".to_string()),
		(Method::POST, "/api/notes/collab/".to_string()),
		(Method::GET, format!("/api/notes/collab/{id}")),
		(Method::GET, format!("/api/notes/collab/access/{id}")),
		(Method::DELETE, format!("/api/notes/collab/revoke/{id}")),
	];
	// a body every POST/PUT handler accepts, so only the missing identity can fail
	let body = json!({ "title": "x", "note_id": id, "guest_id": null, "role": "View" });
	for (method, uri) in routes {
		let (status, _) = call(&app, method.clone(), &uri)
			.json(body.clone())
			.send()
			.await;
		assert_eq!(
			status,
			StatusCode::UNAUTHORIZED,
			"{method} {uri} without X-User-Id"
		);

		let (status, _) = call(&app, method.clone(), &uri)
			.raw_user("not-a-uuid")
			.json(body.clone())
			.send()
			.await;
		assert_eq!(
			status,
			StatusCode::UNAUTHORIZED,
			"{method} {uri} with a malformed X-User-Id"
		);
	}
}

// --- Notes and slugs -------------------------------------------------------------

#[sqlx::test]
async fn a_new_note_gets_a_slug_and_an_empty_document(pool: PgPool) {
	let app = app(pool.clone());
	let alice = user();

	let note = create_note(&app, alice, "My First Note!").await;
	assert_eq!(note["title"], "My First Note!");
	assert_eq!(note["owner_url"], "my-first-note");
	assert_eq!(note["owner_id"], alice.to_string());
	assert_eq!(
		count(
			&pool,
			"SELECT COUNT(*) FROM note_states WHERE note_id = $1",
			&note
		)
		.await,
		1
	);

	let notes = list(&app, alice, "/api/notes").await;
	assert_eq!(notes.len(), 1);
	assert_eq!(notes[0]["id"], note["id"]);
	let (status, fetched) = call(
		&app,
		Method::GET,
		format!("/api/notes/{}", note["id"].as_str().unwrap()),
	)
	.by(alice)
	.send()
	.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(fetched["title"], "My First Note!");
}

#[sqlx::test]
async fn duplicate_titles_get_numbered_slugs_per_owner(pool: PgPool) {
	let app = app(pool);
	let (alice, bob) = (user(), user());

	let slugs: Vec<_> = [
		create_note(&app, alice, "Plan").await,
		create_note(&app, alice, "plan").await,
		create_note(&app, alice, "PLAN!").await,
	]
	.iter()
	.map(|n| n["owner_url"].as_str().unwrap().to_string())
	.collect();
	assert_eq!(slugs, ["plan", "plan-1", "plan-2"]);
	// slugs are unique per owner, not globally
	assert_eq!(create_note(&app, bob, "Plan").await["owner_url"], "plan");

	let (status, by_slug) = call(&app, Method::GET, "/api/notes/u/plan-1")
		.by(alice)
		.send()
		.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(by_slug["title"], "plan");
	let (status, _) = call(&app, Method::GET, "/api/notes/u/plan-1")
		.by(bob)
		.send()
		.await;
	assert_eq!(
		status,
		StatusCode::NOT_FOUND,
		"slug lookup is scoped to the caller"
	);
}

#[sqlx::test]
async fn titles_without_letters_or_digits_still_get_a_usable_slug(pool: PgPool) {
	let app = app(pool);
	let alice = user();
	let first = create_note(&app, alice, "!!!").await;
	let second = create_note(&app, alice, "").await;
	assert_eq!(first["owner_url"], "untitled");
	assert_eq!(second["owner_url"], "untitled-1");
}

#[sqlx::test]
async fn renaming_regenerates_the_slug_without_colliding_with_itself(pool: PgPool) {
	let app = app(pool);
	let alice = user();
	let note = create_note(&app, alice, "Plan").await;

	let (status, same) = rename(&app, alice, &note, "Plan").await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		same["owner_url"], "plan",
		"a note does not collide with its own slug"
	);

	let (_, renamed) = rename(&app, alice, &note, "Road map 2027").await;
	assert_eq!(renamed["title"], "Road map 2027");
	assert_eq!(renamed["owner_url"], "road-map-2027");
	assert!(renamed["updated_at"].as_str() > note["updated_at"].as_str());
}

#[sqlx::test]
async fn titles_longer_than_the_column_are_rejected_not_a_server_error(pool: PgPool) {
	let app = app(pool);
	let alice = user();
	let long = "é".repeat(256); // the limit is in characters, not bytes
	let (status, body) = call(&app, Method::POST, "/api/notes")
		.by(alice)
		.json(json!({ "title": long }))
		.send()
		.await;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert_eq!(
		body["message"],
		"Title is too long (maximum 255 characters)"
	);

	let note = create_note(&app, alice, &"é".repeat(255)).await;
	let (status, _) = rename(&app, alice, &note, &long).await;
	assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn users_cannot_read_rename_or_delete_other_users_notes(pool: PgPool) {
	let app = app(pool);
	let (alice, mallory) = (user(), user());
	let note = create_note(&app, alice, "Private").await;
	let uri = format!("/api/notes/{}", note["id"].as_str().unwrap());

	// 404 rather than 403, so the response does not reveal that the note exists
	let (status, _) = call(&app, Method::GET, &uri).by(mallory).send().await;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) = rename(&app, mallory, &note, "Pwned").await;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) = call(&app, Method::DELETE, &uri).by(mallory).send().await;
	assert_eq!(status, StatusCode::NOT_FOUND);
	assert!(list(&app, mallory, "/api/notes").await.is_empty());

	let (_, still) = call(&app, Method::GET, &uri).by(alice).send().await;
	assert_eq!(still["title"], "Private");
}

#[sqlx::test]
async fn deleting_a_note_removes_its_shares_and_document(pool: PgPool) {
	let app = app(pool.clone());
	let (alice, bob) = (user(), user());
	let note = create_note(&app, alice, "Doomed").await;
	share(&app, alice, &note, Some(bob), "Edit").await;

	let (status, _) = call(
		&app,
		Method::DELETE,
		format!("/api/notes/{}", note["id"].as_str().unwrap()),
	)
	.by(alice)
	.send()
	.await;
	assert_eq!(status, StatusCode::NO_CONTENT);
	assert_eq!(
		count(
			&pool,
			"SELECT COUNT(*) FROM share WHERE note_id = $1",
			&note
		)
		.await,
		0
	);
	assert_eq!(
		count(
			&pool,
			"SELECT COUNT(*) FROM note_states WHERE note_id = $1",
			&note
		)
		.await,
		0
	);
	assert!(
		list(&app, bob, "/api/notes/collab/received")
			.await
			.is_empty()
	);
}

#[sqlx::test]
async fn account_deletion_removes_only_that_users_notes(pool: PgPool) {
	let app = app(pool);
	let (alice, bob) = (user(), user());
	create_note(&app, alice, "One").await;
	create_note(&app, alice, "Two").await;
	create_note(&app, bob, "Bob's").await;

	let (status, _) = call(&app, Method::DELETE, "/api/notes/by-owner")
		.by(alice)
		.send()
		.await;
	assert_eq!(status, StatusCode::NO_CONTENT);
	assert!(list(&app, alice, "/api/notes").await.is_empty());
	assert_eq!(list(&app, bob, "/api/notes").await.len(), 1);
}

#[sqlx::test]
async fn export_contains_only_the_callers_notes_with_their_document_state(pool: PgPool) {
	let app = app(pool);
	let (alice, bob) = (user(), user());
	create_note(&app, alice, "One").await;
	create_note(&app, alice, "Two").await;
	create_note(&app, bob, "Bob's").await;

	let export = list(&app, alice, "/api/notes/export").await;
	let titles: Vec<_> = export
		.iter()
		.map(|n| n["title"].as_str().unwrap())
		.collect();
	assert_eq!(titles, ["One", "Two"]);
	// a fresh note's Yjs state is the empty update [0, 0]
	assert_eq!(export[0]["state_vector"], json!([0, 0]));
}

// --- Sharing ---------------------------------------------------------------------

#[sqlx::test]
async fn only_the_owner_can_share_a_note(pool: PgPool) {
	let app = app(pool);
	let (alice, bob, mallory) = (user(), user(), user());
	let note = create_note(&app, alice, "Shared").await;

	let (status, _) = share(&app, mallory, &note, Some(mallory), "Edit").await;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"a stranger cannot grant themselves access"
	);
	let (status, _) = share(
		&app,
		alice,
		&json!({ "id": Uuid::new_v4() }),
		Some(bob),
		"Edit",
	)
	.await;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) = share(&app, alice, &note, Some(alice), "Edit").await;
	assert_eq!(
		status,
		StatusCode::BAD_REQUEST,
		"owners cannot share with themselves"
	);

	let (status, created) = share(&app, alice, &note, Some(bob), "View").await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(created["access_role"], "View");
	assert_eq!(created["guest_id"], bob.to_string());
	let (status, _) = share(&app, alice, &note, Some(bob), "Edit").await;
	assert_eq!(status, StatusCode::CONFLICT, "one share per guest and note");
}

#[sqlx::test]
async fn guests_see_shared_notes_until_the_share_is_revoked(pool: PgPool) {
	let app = app(pool);
	let (alice, bob, mallory) = (user(), user(), user());
	let note = create_note(&app, alice, "Shared").await;
	let (_, created) = share(&app, alice, &note, Some(bob), "Edit").await;
	let share_id = created["id"].as_str().unwrap();

	let received = list(&app, bob, "/api/notes/collab/received").await;
	assert_eq!(received.len(), 1);
	assert_eq!(received[0]["note_title"], "Shared");
	assert_eq!(received[0]["access_role"], "Edit");
	assert_eq!(received[0]["owner_id"], alice.to_string());

	let access = format!("/api/notes/collab/access/{share_id}");
	let (status, opened) = call(&app, Method::GET, &access).by(bob).send().await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(opened["id"], note["id"]);
	let (status, _) = call(&app, Method::GET, &access).by(mallory).send().await;
	assert_eq!(
		status,
		StatusCode::NOT_FOUND,
		"a share id is not a capability for other users"
	);

	// owner views of the same share
	assert_eq!(
		list(&app, alice, "/api/notes/collab/created").await.len(),
		1
	);
	let collaborators = format!("/api/notes/collab/{}", note["id"].as_str().unwrap());
	assert_eq!(
		list(&app, alice, &collaborators).await[0]["guest_id"],
		bob.to_string()
	);
	assert!(
		list(&app, mallory, &collaborators).await.is_empty(),
		"only the owner lists collaborators"
	);

	let revoke = format!("/api/notes/collab/revoke/{share_id}");
	let (status, _) = call(&app, Method::DELETE, &revoke).by(bob).send().await;
	assert_eq!(status, StatusCode::NOT_FOUND, "guests cannot revoke");
	let (status, _) = call(&app, Method::DELETE, &revoke).by(alice).send().await;
	assert_eq!(status, StatusCode::NO_CONTENT);

	assert!(
		list(&app, bob, "/api/notes/collab/received")
			.await
			.is_empty()
	);
	let (status, _) = call(&app, Method::GET, &access).by(bob).send().await;
	assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn a_public_link_opens_for_any_signed_in_user(pool: PgPool) {
	let app = app(pool);
	let (alice, anyone) = (user(), user());
	let note = create_note(&app, alice, "Public").await;

	let (status, link) = share(&app, alice, &note, None, "View").await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(link["guest_id"], Value::Null);
	let (status, opened) = call(
		&app,
		Method::GET,
		format!("/api/notes/collab/access/{}", link["id"].as_str().unwrap()),
	)
	.by(anyone)
	.send()
	.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(opened["title"], "Public");

	let (status, _) = share(&app, alice, &note, None, "Edit").await;
	assert_eq!(
		status,
		StatusCode::CONFLICT,
		"one public link per note (UNIQUE NULLS NOT DISTINCT)"
	);
}

// --- i18n --------------------------------------------------------------------------

#[sqlx::test]
async fn error_messages_follow_accept_language(pool: PgPool) {
	let app = app(pool);
	let missing = format!("/api/notes/{}", Uuid::new_v4());
	let message = |lang: &'static str| {
		let app = app.clone();
		let missing = missing.clone();
		async move {
			let (status, body) = call(&app, Method::GET, missing)
				.by(user())
				.lang(lang)
				.send()
				.await;
			assert_eq!(status, StatusCode::NOT_FOUND);
			body["message"].as_str().unwrap().to_string()
		}
	};

	assert_eq!(
		message("de-DE,de;q=0.9,en;q=0.8").await,
		"Notiz nicht gefunden"
	);
	assert_eq!(
		message("de").await,
		"Notiz nicht gefunden",
		"language-only tags match a regional bundle"
	);
	assert_eq!(message("el-GR").await, "Η σημείωση δεν βρέθηκε");
	assert_eq!(
		message("fr-FR").await,
		"Note not found",
		"unknown languages fall back to English"
	);
}
