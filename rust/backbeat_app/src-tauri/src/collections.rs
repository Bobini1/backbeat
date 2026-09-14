//! Manage installed collection documents.

use std::{collections::HashMap, sync::Arc};

use backbeat_core::CollectionKind;
use backbeat_sdk::{
	Backbeat, DataId,
	collections::{
		CollectionDownloadDataReport, CollectionMetadata, CourseContents, PackContents,
		TableContents,
	},
};
use futures::future::join_all;
use serde::Serialize;
use sqlx::types::chrono::{DateTime, Utc};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

const COLLECTION_DOWNLOAD_EVENT: &str = "collection-download";

#[derive(Debug, Serialize)]
pub struct CollectionDocumentStatus {
	pub kind: CollectionKind,
	pub url: String,
	pub updated: Option<DateTime<Utc>>,
	pub needs_update: bool,
	pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionDownloadEvent {
	pub kind: CollectionKind,
	pub name: String,
	pub url: String,
	pub state: CollectionDownloadState,
	pub current: u64,
	pub total: u64,
	pub item: Option<DataId>,
	pub description: Option<String>,
	pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionDownloadState {
	Downloading,
	Done,
	Failed,
}

fn collection_records(
	store: &Backbeat,
	kind: CollectionKind,
) -> Result<Vec<CollectionMetadata>, String> {
	match kind {
		CollectionKind::Table => store.list_tables(&[]),
		CollectionKind::Pack => store.list_packs(&[]),
		CollectionKind::Course => store.list_courses(&[]),
	}
	.map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn list_installed_collection_documents(
	state: State<'_, AppState>,
	kind: CollectionKind,
) -> Result<Vec<CollectionMetadata>, String> {
	let store = state.store.get()?;
	collection_records(&store, kind)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_table(state: State<'_, AppState>, url: String) -> Result<TableContents, String> {
	let store = state.store.get()?;
	store.get_table(&url).map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_pack(state: State<'_, AppState>, url: String) -> Result<PackContents, String> {
	let store = state.store.get()?;
	store.get_pack(&url).map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_course(state: State<'_, AppState>, url: String) -> Result<CourseContents, String> {
	let store = state.store.get()?;
	store.get_course(&url).map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state, app))]
pub async fn download_collection_content(
	app: AppHandle,
	state: State<'_, AppState>,
	url: String,
	kind: CollectionKind,
	name: String,
) -> Result<(), String> {
	spawn_collection_download(app, state.store.get()?, url, kind, name);
	Ok(())
}

fn spawn_collection_download(
	app: AppHandle,
	store: Arc<Backbeat>,
	url: String,
	kind: CollectionKind,
	name: String,
) {
	tauri::async_runtime::spawn(async move {
		run_collection_download(&app, &store, url, kind, name).await;
	});
}

async fn run_collection_download(
	app: &AppHandle,
	store: &Backbeat,
	url: String,
	kind: CollectionKind,
	name: String,
) {
	let descriptions = collection_item_descriptions(store, kind, &url).unwrap_or_default();
	let progress_app = app.clone();
	let progress_url = url.clone();
	let progress_name = name.clone();
	let result = store
		.collection_fetch_download_data(&url, move |progress| {
			let description = descriptions.get(&progress.item).cloned();
			let _ = progress_app.emit(
				COLLECTION_DOWNLOAD_EVENT,
				CollectionDownloadEvent {
					kind,
					name: progress_name.clone(),
					url: progress_url.clone(),
					state: CollectionDownloadState::Downloading,
					current: progress.current,
					total: progress.total,
					item: Some(progress.item),
					description,
					error: None,
				},
			);
		})
		.await;

	let event = match result {
		Ok(report) => {
			let total = collection_download_total(&report);
			let failed = report.has_failures();
			CollectionDownloadEvent {
				kind,
				name,
				url,
				state: if failed {
					CollectionDownloadState::Failed
				} else {
					CollectionDownloadState::Done
				},
				current: total,
				total,
				item: None,
				description: None,
				error: failed.then(|| format!("{} downloads failed", report.errors.len())),
			}
		}
		Err(err) => {
			tracing::error!(?err, %kind, "collection content download failed");
			CollectionDownloadEvent {
				kind,
				name,
				url,
				state: CollectionDownloadState::Failed,
				current: 0,
				total: 0,
				item: None,
				description: None,
				error: Some(err.to_string()),
			}
		}
	};
	let _ = app.emit(COLLECTION_DOWNLOAD_EVENT, event);
}

fn collection_item_descriptions(
	store: &Backbeat,
	kind: CollectionKind,
	url: &str,
) -> Result<HashMap<DataId, String>, backbeat_sdk::StoreError> {
	match kind {
		CollectionKind::Table => Ok(store
			.get_table(url)?
			.levels
			.into_iter()
			.flat_map(|level| level.charts)
			.map(|chart| (DataId::Chart(chart.id), chart.desc))
			.collect()),
		CollectionKind::Course => Ok(store
			.get_course(url)?
			.charts
			.into_iter()
			.map(|chart| (DataId::Chart(chart.id), chart.desc))
			.collect()),
		CollectionKind::Pack => Ok(store
			.get_pack(url)?
			.bundles
			.into_iter()
			.map(|bundle| (DataId::Bundle(bundle.id), bundle.desc))
			.collect()),
	}
}

fn collection_download_total(report: &CollectionDownloadDataReport) -> u64 {
	report.charts_downloaded
		+ report.charts_skipped
		+ report.charts_failed
		+ report.bundles_downloaded
		+ report.bundles_skipped
		+ report.bundles_failed
		+ report.assets_downloaded
		+ report.assets_skipped
		+ report.assets_failed
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn check_installed_collection_updates(
	state: State<'_, AppState>,
	kind: CollectionKind,
) -> Result<Vec<CollectionDocumentStatus>, String> {
	let store = state.store.get()?;
	let records = collection_records(&store, kind)?;

	let futures = records.into_iter().map(|record| {
		let store = store.clone();
		async move { check_one_collection_update(&store, kind, record.url, record.updated).await }
	});

	join_all(futures).await.into_iter().collect()
}

async fn check_one_collection_update(
	store: &Backbeat,
	kind: CollectionKind,
	url: String,
	updated: DateTime<Utc>,
) -> Result<CollectionDocumentStatus, String> {
	match store.collection_fetch_header(&url).await {
		Ok(header) => {
			if header.kind != kind {
				return Ok(CollectionDocumentStatus {
					kind,
					url,
					updated: None,
					needs_update: false,
					error: Some(format!(
						"collection header kind is {}",
						header.kind.as_str()
					)),
				});
			}

			let stored_updated = updated;

			let needs_update = header.timestamp > stored_updated;
			Ok(CollectionDocumentStatus {
				kind,
				url,
				updated: Some(header.timestamp),
				needs_update,
				error: None,
			})
		}
		Err(err) => Ok(CollectionDocumentStatus {
			kind,
			url,
			updated: None,
			needs_update: false,
			error: Some(err.to_string()),
		}),
	}
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn install_collection_document(
	state: State<'_, AppState>,
	url: String,
) -> Result<CollectionKind, String> {
	let store = state.store.get()?;
	let result = store
		.collection_fetch_upsert(&url)
		.await
		.map_err(|err| err.to_string())?;
	Ok(result.kind)
}

pub async fn install_or_update_collection(store: &Backbeat, url: &str) -> Result<(), String> {
	store
		.collection_fetch_upsert(url)
		.await
		.map(|_| ())
		.map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remove_collection(
	state: State<'_, AppState>,
	url: String,
	remove_charts_too: bool,
) -> Result<(), String> {
	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		store
			.collection_rm(&url, remove_charts_too)
			.map(|_| ())
			.map_err(|err| err.to_string())
	})
	.await
	.map_err(|err| {
		tracing::error!(?err, "collection removal task panicked");
		err.to_string()
	})?
}
