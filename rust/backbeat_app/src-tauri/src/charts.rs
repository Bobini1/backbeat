//! Chart catalogue commands.

use std::path::{Path, PathBuf};

use backbeat_core::{BackbeatFile, BundleId};
use backbeat_sdk::bundle::{detail::BundleDetail, search::BundleSearchResult};
use tauri::{AppHandle, Manager, State};

use crate::AppState;

const CHART_SEARCH_DEFAULT_LIMIT: u32 = 50;
const CHART_SEARCH_MAX_LIMIT: u32 = 100;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn search_charts(
	state: State<'_, AppState>,
	query: Option<String>,
	offset: u64,
	limit: Option<u32>,
	extension: Option<String>,
) -> Result<BundleSearchResult, String> {
	let limit = limit
		.unwrap_or(CHART_SEARCH_DEFAULT_LIMIT)
		.min(CHART_SEARCH_MAX_LIMIT);
	let store = state.store.get()?;
	let extensions: Vec<_> = extension.as_deref().into_iter().collect();
	let result = store
		.search_bundles(query.as_deref(), offset, limit, &extensions)
		.map_err(|err| {
			tracing::error!(?err, "bundle search failed");
			err.to_string()
		})?;

	tracing::debug!(
		returned = result.charts.len(),
		has_more = result.has_more,
		offset,
		"chart search complete"
	);

	Ok(result)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn list_available_extensions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
	let store = state.store.get()?;
	let pool = sqlx::sqlite::SqlitePoolOptions::new()
		.max_connections(1)
		.connect(&store.sqlite_connection_url())
		.await
		.map_err(|err| err.to_string())?;

	sqlx::query_scalar!(
		"SELECT DISTINCT extension AS \"extension!: String\" FROM bundle \
		 WHERE extension IS NOT NULL ORDER BY extension"
	)
	.fetch_all(&pool)
	.await
	.map_err(|err| err.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_bundle_detail(
	state: State<'_, AppState>,
	bundle_id: BundleId,
) -> Result<BundleDetail, String> {
	let store = state.store.get()?;

	let detail = store.bundle_detail(bundle_id).map_err(|err| {
		tracing::error!(?err, %bundle_id, "chart detail query failed");
		err.to_string()
	})?;

	Ok(detail)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn remove_bundle(state: State<'_, AppState>, bundle_id: BundleId) -> Result<(), String> {
	let store = state.store.get()?;
	store.bundle_rm(bundle_id).map_err(|err| {
		tracing::error!(?err, %bundle_id, "chart removal failed");
		err.to_string()
	})
}

#[tauri::command]
#[tracing::instrument(skip(state, paths))]
pub async fn import_files(state: State<'_, AppState>, paths: Vec<String>) -> Result<usize, String> {
	let store = state.store.get()?;
	let imported = paths.len();

	tokio::task::spawn_blocking(move || {
		for path in paths {
			let path = Path::new(&path);
			match path
				.extension()
				.and_then(|extension| extension.to_str())
				.map(str::to_ascii_lowercase)
				.as_deref()
			{
				Some("bb") => {
					let bb = BackbeatFile::from_file(path).map_err(|err| {
						tracing::error!(?err, ?path, "failed to read bundle import");
						format!("Failed to import {}: {err}", path.display())
					})?;
					store.import_bundle(&bb).map_err(|err| {
						tracing::error!(?err, ?path, "failed to import bundle");
						format!("Failed to import {}: {err}", path.display())
					})?;
				}
				Some("bbzip") => store.import_bbzip(path).map_err(|err| {
					tracing::error!(?err, ?path, "failed to import bundle archive");
					format!("Failed to import {}: {err}", path.display())
				})?,
				_ => return Err(format!("{} is not a .bb or .bbzip file", path.display())),
			}
		}

		Ok(imported)
	})
	.await
	.map_err(|err| {
		tracing::error!(?err, "manual import task panicked");
		err.to_string()
	})?
}

#[tauri::command]
#[tracing::instrument(skip(app, state))]
pub async fn export_bundle(
	app: AppHandle,
	state: State<'_, AppState>,
	bundle_id: BundleId,
	bbzip: bool,
) -> Result<(), String> {
	let store = state.store.get()?;
	let documents_dir = app.path().document_dir().map_err(|err| {
		tracing::error!(?err, "failed to locate Documents directory");
		err.to_string()
	})?;

	tokio::task::spawn_blocking(move || {
		let detail = store.bundle_detail(bundle_id).map_err(|err| {
			tracing::error!(?err, %bundle_id, "failed to name chart export");
			err.to_string()
		})?;
		let output = export_output_path(&documents_dir, &detail.description, bbzip);

		store
			.export_bundle(bundle_id, &output, bbzip)
			.map_err(|err| {
				tracing::error!(?err, %bundle_id, ?output, bbzip, "chart export failed");
				format!("Export failed: {err}")
			})?;

		let folder = if bbzip {
			output.parent().unwrap_or(&output)
		} else {
			&output
		};
		open::that_detached(folder).map_err(|err| {
			tracing::error!(?err, ?folder, "failed to open chart export folder");
			format!(
				"Exported successfully, but couldn't open {}: {err}",
				folder.display()
			)
		})
	})
	.await
	.map_err(|err| {
		tracing::error!(?err, "chart export task panicked");
		err.to_string()
	})?
}

fn export_output_path(documents_dir: &Path, description: &str, bbzip: bool) -> PathBuf {
	let root = documents_dir.join("Backbeat Exports").join("Charts");
	let name = sanitise_export_name(description);
	if bbzip {
		root.join(format!("{name}.bbzip"))
	} else {
		root.join(name)
	}
}

fn sanitise_export_name(name: &str) -> String {
	let mut name: String = name
		.chars()
		.map(|character| {
			if character.is_control()
				|| matches!(
					character,
					'/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
				) {
				'-'
			} else {
				character
			}
		})
		.collect();
	name = name.trim_end_matches([' ', '.']).to_owned();
	if name.is_empty() || matches!(name.as_str(), "." | "..") {
		return "chart".to_owned();
	}

	let stem = name
		.split('.')
		.next()
		.unwrap_or_default()
		.trim_end_matches([' ', '.']);
	if is_windows_device_name(stem) {
		let index = name.find('.').unwrap_or(name.len());
		name.insert(index, '-');
	}

	name
}

fn is_windows_device_name(stem: &str) -> bool {
	let stem = stem.to_ascii_uppercase();
	matches!(
		stem.as_str(),
		"CON"
			| "PRN" | "AUX"
			| "NUL" | "COM1"
			| "COM2" | "COM3"
			| "COM4" | "COM5"
			| "COM6" | "COM7"
			| "COM8" | "COM9"
			| "LPT1" | "LPT2"
			| "LPT3" | "LPT4"
			| "LPT5" | "LPT6"
			| "LPT7" | "LPT8"
			| "LPT9" | "CLOCK$"
	)
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::export_output_path;

	#[test]
	fn chart_exports_use_portable_paths_under_documents() {
		let documents = Path::new("Documents");
		assert_eq!(
			export_output_path(documents, "A/B: C?", false),
			documents.join("Backbeat Exports/Charts/A-B- C-")
		);
		assert_eq!(
			export_output_path(documents, "A/B: C?", true),
			documents.join("Backbeat Exports/Charts/A-B- C-.bbzip")
		);
		assert_eq!(
			export_output_path(documents, "...", false),
			documents.join("Backbeat Exports/Charts/chart")
		);
		assert_eq!(
			export_output_path(documents, "CON.txt", true),
			documents.join("Backbeat Exports/Charts/CON-.txt.bbzip")
		);
	}
}
