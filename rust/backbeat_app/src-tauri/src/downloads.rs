//! Tauri commands for installing a chart or bundle from the configured
//! remotes. The GUI frontend can invoke these directly (e.g. from a future
//! "download" button); the deep-link handler in [`crate::deep_links`] calls
//! the underlying store methods itself rather than routing through these.

use std::collections::HashMap;

use backbeat_core::{BundleId, ChartId};
use backbeat_sdk::{
	Backbeat, DataId,
	download_manager::{DownloadListResult as SdkDownloadListResult, DownloadOverview},
};
use serde::Serialize;
use sqlx::Connection;
use tauri::State;

use crate::AppState;

const DOWNLOAD_LIST_DEFAULT_LIMIT: u32 = 50;
const DOWNLOAD_LIST_MAX_LIMIT: u32 = 200;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadListResult {
	#[serde(flatten)]
	pub downloads: SdkDownloadListResult,
	pub descriptions: HashMap<String, String>,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn download_chart(state: State<'_, AppState>, chart_id: ChartId) -> Result<(), String> {
	let store = state.store.get()?;
	store.server_download_chart(&chart_id).await.map_err(|err| {
		tracing::error!(?err, %chart_id, "chart download failed");
		err.to_string()
	})
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn download_bundle(
	state: State<'_, AppState>,
	bundle_id: BundleId,
) -> Result<(), String> {
	let store = state.store.get()?;

	store
		.server_download_bundle(bundle_id)
		.await
		.map_err(|err| {
			tracing::error!(?err, %bundle_id, "bundle install failed");
			err.to_string()
		})
}

/// Lightweight aggregate of download manager state for ambient UI.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn download_overview(state: State<'_, AppState>) -> Result<DownloadOverview, String> {
	let store = state.store.get()?;
	Ok(store.download_overview())
}

/// Paginated download list. Defaults to [`DOWNLOAD_LIST_DEFAULT_LIMIT`].
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn list_downloads(
	state: State<'_, AppState>,
	offset: u64,
	limit: Option<u32>,
) -> Result<DownloadListResult, String> {
	let limit = limit
		.unwrap_or(DOWNLOAD_LIST_DEFAULT_LIMIT)
		.min(DOWNLOAD_LIST_MAX_LIMIT);
	let store = state.store.get()?;
	let result = store.collection_download_list(offset, limit);
	let ids = result
		.downloads
		.iter()
		.filter_map(|download| collection_item_id(&download.key))
		.collect();
	let descriptions = match collection_descriptions(&store, ids).await {
		Ok(descriptions) => descriptions,
		Err(err) => {
			tracing::warn!(?err, "could not load collection descriptions for downloads");
			HashMap::new()
		}
	};

	Ok(DownloadListResult {
		downloads: result,
		descriptions,
	})
}

async fn collection_descriptions(
	store: &Backbeat,
	download_ids: Vec<String>,
) -> Result<HashMap<String, String>, sqlx::Error> {
	if download_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let download_ids = serde_json::to_string(&download_ids).expect("download IDs must serialize");
	let mut connection = sqlx::SqliteConnection::connect(&store.sqlite_connection_url()).await?;
	let rows = sqlx::query!(
		r#"
		SELECT
			collection_item.id AS "id!: String",
			collection_item.description AS "description!: String"
		FROM (
			SELECT id, desc AS description FROM difftable_chart

			UNION ALL

			SELECT id, desc AS description FROM course_chart

			UNION ALL

			SELECT bundle_id AS id, desc AS description FROM pack_entry
		) collection_item
		WHERE collection_item.id IN (SELECT value FROM json_each(?1))
		GROUP BY collection_item.id
		"#,
		download_ids,
	)
	.fetch_all(&mut connection)
	.await?;

	Ok(rows
		.into_iter()
		.filter_map(|row| {
			let description = row.description.trim();
			(!description.is_empty()).then(|| (row.id, description.to_owned()))
		})
		.collect())
}

pub(crate) async fn collection_description(store: &Backbeat, item: &DataId) -> Option<String> {
	let id = collection_item_id(item)?;
	collection_descriptions(store, vec![id.clone()])
		.await
		.ok()?
		.remove(&id)
}

fn collection_item_id(item: &DataId) -> Option<String> {
	match item {
		DataId::Bundle(bundle_id) => Some(bundle_id.to_string()),
		DataId::Chart(chart_id) => Some(chart_id.to_string()),
		DataId::Asset(_) => None,
	}
}

/// Cancel a single in-flight download by its manager key. Returns `true` if a
/// download was actively cancelled.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn cancel_download(state: State<'_, AppState>, data: DataId) -> Result<bool, String> {
	let store = state.store.get()?;
	let cancelled = store.download_cancel(data);

	Ok(cancelled)
}

/// Remove every finished download from the manager. Returns the number of
/// entries removed.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn clear_finished_downloads(state: State<'_, AppState>) -> Result<usize, String> {
	let store = state.store.get()?;
	let cancelled = store.download_clear_finished();

	Ok(cancelled)
}
