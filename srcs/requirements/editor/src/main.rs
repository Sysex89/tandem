mod db;
mod metrics;
mod models;
mod state;
mod sync;
mod title;
mod ws_handler;

use axum::{
	Router,
	http::{StatusCode, header},
	response::{IntoResponse, Response},
	routing::get,
};
use sqlx::PgPool;
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
	// Initialize tracing
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "editor=debug,tower_http=debug,axum::rejection=trace".into()),
		)
		.with(tracing_subscriber::fmt::layer())
		.init();

	metrics::init();
	let db_pool = setup_database().await;
	let app_state = Arc::new(AppState::new(db_pool));

	let app = Router::new()
		.route("/ws/{id}", get(ws_handler::ws_route))
		// Internal only: nginx proxies /ws/ but not /metrics; Prometheus scrapes it directly.
		.route("/metrics", get(metrics_handler))
		.with_state(app_state);

	let port = std::env::var("PORT").unwrap_or_else(|_| "3004".to_string());
	let addr: SocketAddr = format!("0.0.0.0:{}", port)
		.parse()
		.expect("Invalid address");

	tracing::info!("Listening on {}", addr);
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	axum::serve(listener, app).await.unwrap();
}

async fn setup_database() -> PgPool {
	let user = std::env::var("DB_USER").expect("DB_USER must be set");
	let password = std::env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");
	let db_name = std::env::var("DB_NAME").expect("DB_NAME must be set");
	let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "postgres".to_string());
	let db_url = format!("postgresql://{}:{}@{}/{}", user, password, db_host, db_name);

	tracing::info!("Connecting to database at {}:5432/{}", db_host, db_name);

	PgPool::connect(&db_url)
		.await
		.expect("Failed to create pool.")
}

async fn metrics_handler() -> Response {
	match metrics::gather_prometheus_text() {
		Ok((body, content_type)) => ([(header::CONTENT_TYPE, content_type)], body).into_response(),
		Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
	}
}
