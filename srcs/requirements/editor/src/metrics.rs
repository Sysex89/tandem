use prometheus::{
	Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
	IntGaugeVec, Opts, TextEncoder, exponential_buckets,
};
use std::sync::OnceLock;

/// Exposition format for `TextEncoder` (stable, does not depend on encoder instance).
const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

pub struct Metrics {
	/// Documents currently loaded in memory (one room per open note)
	pub rooms_active: IntGauge,
	/// Open WebSocket connections, by access level (read_write | read_only)
	pub clients_connected: IntGaugeVec,
	/// WebSocket upgrade attempts, by result (accepted | unauthorized | forbidden)
	pub connections_total: IntCounterVec,
	/// y-sync messages received from clients, by kind
	pub messages_total: IntCounterVec,
	/// Document updates from read-only clients that were dropped
	pub updates_dropped_total: IntCounter,
	/// Open connections closed because access was revoked
	pub access_revoked_total: IntCounter,
	/// Document saves to Postgres, by trigger (periodic | last_client) and result (ok | error)
	pub saves_total: IntCounterVec,
	pub save_duration_seconds: HistogramVec,
	/// Encoded Yjs state size at save time
	pub document_size_bytes: Histogram,
	/// Title pushes to the notes service, by result (ok | rejected | error)
	pub title_syncs_total: IntCounterVec,
}

fn register<T: prometheus::core::Collector + Clone + 'static>(metric: T) -> T {
	prometheus::default_registry()
		.register(Box::new(metric.clone()))
		.expect("register editor metric");
	metric
}

pub fn get() -> &'static Metrics {
	static M: OnceLock<Metrics> = OnceLock::new();
	M.get_or_init(|| Metrics {
		rooms_active: register(
			IntGauge::new("editor_rooms_active", "Documents loaded in memory.").unwrap(),
		),
		clients_connected: register(
			IntGaugeVec::new(
				Opts::new("editor_clients_connected", "Open WebSocket connections."),
				&["access"],
			)
			.unwrap(),
		),
		connections_total: register(
			IntCounterVec::new(
				Opts::new("editor_connections_total", "WebSocket upgrade attempts."),
				&["result"],
			)
			.unwrap(),
		),
		messages_total: register(
			IntCounterVec::new(
				Opts::new(
					"editor_messages_total",
					"y-sync messages received from clients.",
				),
				&["kind"],
			)
			.unwrap(),
		),
		updates_dropped_total: register(
			IntCounter::new(
				"editor_updates_dropped_total",
				"Document updates from read-only clients that were dropped.",
			)
			.unwrap(),
		),
		access_revoked_total: register(
			IntCounter::new(
				"editor_access_revoked_total",
				"Open connections closed because access was revoked.",
			)
			.unwrap(),
		),
		saves_total: register(
			IntCounterVec::new(
				Opts::new("editor_saves_total", "Document saves to Postgres."),
				&["trigger", "result"],
			)
			.unwrap(),
		),
		save_duration_seconds: register(
			HistogramVec::new(
				HistogramOpts::new(
					"editor_save_duration_seconds",
					"Time to persist a document.",
				)
				.buckets(vec![
					0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
				]),
				&["trigger"],
			)
			.unwrap(),
		),
		document_size_bytes: register(
			Histogram::with_opts(
				HistogramOpts::new(
					"editor_document_size_bytes",
					"Encoded document size at save time.",
				)
				// 256 B .. ~4 MB
				.buckets(exponential_buckets(256.0, 4.0, 8).unwrap()),
			)
			.unwrap(),
		),
		title_syncs_total: register(
			IntCounterVec::new(
				Opts::new(
					"editor_title_syncs_total",
					"Title pushes to the notes service.",
				),
				&["result"],
			)
			.unwrap(),
		),
	})
}

/// Registers every metric up front so /metrics lists them before first use.
pub fn init() {
	let m = get();
	for access in ["read_write", "read_only"] {
		m.clients_connected.with_label_values(&[access]);
	}
	for result in ["accepted", "unauthorized", "forbidden"] {
		m.connections_total.with_label_values(&[result]);
	}
}

pub fn access_label(can_write: bool) -> &'static str {
	if can_write { "read_write" } else { "read_only" }
}

/// Prometheus text exposition for the default registry.
pub fn gather_prometheus_text() -> Result<(String, &'static str), prometheus::Error> {
	let encoder = TextEncoder::new();
	let mut buf = vec![];
	encoder.encode(&prometheus::gather(), &mut buf)?;
	Ok((
		String::from_utf8_lossy(&buf).into_owned(),
		PROMETHEUS_CONTENT_TYPE,
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_editor_metrics_are_exposed() {
		init();
		let (text, content_type) = gather_prometheus_text().unwrap();
		assert!(content_type.starts_with("text/plain"));
		for name in [
			"editor_rooms_active",
			"editor_clients_connected",
			"editor_connections_total",
			"editor_updates_dropped_total",
			"editor_access_revoked_total",
		] {
			assert!(text.contains(name), "{name} missing from /metrics");
		}
	}
}
