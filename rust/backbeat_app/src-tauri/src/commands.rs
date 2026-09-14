//! `#[tauri::command]` handlers exposed to the frontend over the IPC bridge.

use std::time::Instant;

use backbeat_sdk::store::stats::StoreStats;
use serde::Serialize;
use tauri::State;

use crate::AppState;

/// Frontend startup timing mark, sent by the webview during boot.
#[tauri::command]
#[tracing::instrument]
pub fn startup_mark(label: String, elapsed_ms: f64) {
	tracing::debug!(
		label = %label,
		frontend_elapsed_ms = elapsed_ms,
		"startup timing: frontend mark"
	);
}

/// A log line forwarded from the frontend (`main.tsx` wraps `console.*` and the
/// global `error`/`unhandledrejection` handlers to call this), so JS-side
/// errors land in the same rotating log file as Rust-side `tracing` events.
#[tauri::command]
pub fn frontend_log(level: String, message: String, location: Option<String>) {
	let location = location.as_deref();
	match level.as_str() {
		"error" => tracing::error!(%message, location = ?location, "frontend"),
		"warn" => tracing::warn!(%message, location = ?location, "frontend"),
		_ => tracing::info!(%message, location = ?location, "frontend"),
	}
}

/// Build metadata shown in the shell sidebar.
#[derive(Serialize)]
pub struct BuildInfo {
	pub version: String,
	pub git_sha_short: Option<String>,
}

#[tauri::command]
pub fn build_info() -> BuildInfo {
	BuildInfo {
		version: env!("CARGO_PKG_VERSION").to_string(),
		git_sha_short: option_env!("VERGEN_GIT_SHA").map(|sha| {
			let short_len = sha.len().min(7);
			sha[..short_len].to_string()
		}),
	}
}

/// Open an http(s) URL in the user's default browser.
#[tauri::command]
#[tracing::instrument]
pub fn open_url(url: String) -> Result<(), String> {
	let parsed = url::Url::parse(url.trim()).map_err(|err| format!("invalid URL: {err}"))?;
	match parsed.scheme() {
		"http" | "https" => {}
		scheme => return Err(format!("refusing to open {scheme} URL")),
	}
	open::that_detached(parsed.as_str()).map_err(|err| {
		tracing::error!(?err, %url, "failed to open URL");
		err.to_string()
	})
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn store_summary(state: State<'_, AppState>) -> Result<StoreStats, String> {
	let started = Instant::now();
	tracing::debug!("fetching store summary");

	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		let store_stats = store.stats().map_err(|err| {
			tracing::error!(?err, "store stats query failed");
			err.to_string()
		})?;

		tracing::debug!(
			charts = store_stats.charts,
			tables = store_stats.tables,
			courses = store_stats.courses,
			packs = store_stats.packs,
			asset_count = store_stats.asset_count,
			total_bytes = store_stats.total_bytes(),
			"store summary ready"
		);
		tracing::debug!(
			elapsed_ms = started.elapsed().as_millis(),
			"startup timing: store_summary ready"
		);

		Ok(store_stats)
	})
	.await
	.map_err(|err| {
		tracing::error!(?err, "store summary task panicked");
		err.to_string()
	})?
}
