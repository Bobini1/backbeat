//! Read and write global `backbeat.toml` settings from the GUI.
use std::path::Path;

use backbeat_sdk::maintenance::check::CorruptionReport;
use backbeat_store_config::{
	BackbeatConfig, ByteSize, CONFIG_FILENAME, MAX_DOWNLOAD_CONCURRENCY, default_config_dir,
};
use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Serialize)]
pub struct SettingsSnapshot {
	pub config_file: String,
	pub log_dir: String,
	pub store_inline: String,
	pub download_concurrency: u32,
	pub download_stream: String,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsSnapshot, String> {
	let config = load_config()?;
	let config_dir = default_config_dir();

	Ok(SettingsSnapshot {
		config_file: config_dir.join(CONFIG_FILENAME).display().to_string(),
		log_dir: state.store.get()?.logs_dir().display().to_string(),
		store_inline: config.store.inline.to_string(),
		download_concurrency: config.downloads.concurrency,
		download_stream: config.downloads.stream.to_string(),
	})
}

fn open_folder(dir: &Path) -> Result<(), String> {
	std::fs::create_dir_all(dir).map_err(|err| {
		tracing::error!(?err, dir = %dir.display(), "failed to create directory");
		err.to_string()
	})?;

	open::that_detached(dir).map_err(|err| {
		tracing::error!(?err, dir = %dir.display(), "failed to open directory");
		err.to_string()
	})
}

#[tauri::command]
#[tracing::instrument]
pub fn open_config_folder() -> Result<(), String> {
	open_folder(&default_config_dir())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn open_logs_folder(state: State<'_, AppState>) -> Result<(), String> {
	open_folder(&state.store.get()?.logs_dir())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn save_settings(
	state: State<'_, AppState>,
	store_inline: String,
	download_concurrency: u32,
	download_stream: String,
) -> Result<SettingsSnapshot, String> {
	let dir = default_config_dir();
	let mut config = load_config()?;

	config.store.inline = ByteSize::parse(store_inline.trim()).map_err(|err| {
		tracing::debug!(?err, "invalid store.inline value");
		err.to_string()
	})?;

	if download_concurrency == 0 {
		return Err("downloads.concurrency must be at least 1".into());
	}
	if download_concurrency > MAX_DOWNLOAD_CONCURRENCY {
		return Err(format!(
			"downloads.concurrency must be at most {MAX_DOWNLOAD_CONCURRENCY}"
		));
	}
	config.downloads.concurrency = download_concurrency;

	config.downloads.stream = ByteSize::parse(download_stream.trim()).map_err(|err| {
		tracing::debug!(?err, "invalid downloads.stream value");
		err.to_string()
	})?;
	config.write_to_dir(&dir).map_err(|err| {
		tracing::error!(?err, "failed to persist settings");
		err.to_string()
	})?;

	tracing::info!("settings saved");

	Ok(snapshot_from_config(
		&config,
		&state.store.get()?.logs_dir(),
	))
}

fn snapshot_from_config(config: &BackbeatConfig, log_dir: &Path) -> SettingsSnapshot {
	let config_dir = default_config_dir();
	SettingsSnapshot {
		config_file: config_dir.join(CONFIG_FILENAME).display().to_string(),
		log_dir: log_dir.display().to_string(),
		store_inline: config.store.inline.to_string(),
		download_concurrency: config.downloads.concurrency,
		download_stream: config.downloads.stream.to_string(),
	}
}

fn load_config() -> Result<BackbeatConfig, String> {
	BackbeatConfig::load_with_overridden_dir(&default_config_dir()).map_err(|err| {
		tracing::error!(?err, "failed to load backbeat config");
		err.to_string()
	})
}

#[derive(Serialize)]
pub struct CorruptionCheckResponse {
	pub is_ok: bool,
	pub issue_count: usize,
	pub large_asset_count: usize,
	pub chart_count: usize,
	pub chart_id_count: usize,
	pub missing_assets: usize,
	pub corrupt_assets: usize,
	pub corrupt_charts: usize,
	pub wrong_chart_ids: usize,
	pub uncomputable_chart_ids: usize,
	pub dangling_asset_refs: usize,
}

impl From<CorruptionReport> for CorruptionCheckResponse {
	fn from(report: CorruptionReport) -> Self {
		Self {
			is_ok: report.is_ok(),
			issue_count: report.issue_count(),
			large_asset_count: report.large_asset_count,
			chart_count: report.chart_count,
			chart_id_count: report.chart_id_count,
			missing_assets: report.missing_assets.len(),
			corrupt_assets: report.corrupt_assets.len(),
			corrupt_charts: report.corrupt_charts.len(),
			wrong_chart_ids: report.wrong_chart_ids.len(),
			uncomputable_chart_ids: report.uncomputable_chart_ids.len(),
			dangling_asset_refs: report.dangling_asset_refs.len(),
		}
	}
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn asset_prune(state: State<'_, AppState>) -> Result<u64, String> {
	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		store.asset_prune(false).map_err(|err| {
			tracing::error!(?err, "asset prune failed");
			err.to_string()
		})
	})
	.await
	.map_err(|err| err.to_string())?
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn disk_prune(state: State<'_, AppState>) -> Result<u64, String> {
	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		store.disk_prune(false).map_err(|err| {
			tracing::error!(?err, "disk prune failed");
			err.to_string()
		})
	})
	.await
	.map_err(|err| err.to_string())?
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn corruption_check(
	state: State<'_, AppState>,
) -> Result<CorruptionCheckResponse, String> {
	let store = state.store.get()?;
	let report = tokio::task::spawn_blocking(move || {
		store.corruption_check().map_err(|err| {
			tracing::error!(?err, "corruption check failed");
			err.to_string()
		})
	})
	.await
	.map_err(|err| err.to_string())??;
	Ok(report.into())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn corruption_repair(state: State<'_, AppState>) -> Result<(), String> {
	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		store.corruption_repair().map_err(|err| {
			tracing::error!(?err, "corruption repair failed");
			err.to_string()
		})
	})
	.await
	.map_err(|err| err.to_string())?
}
