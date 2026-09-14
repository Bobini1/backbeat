//! Asset detail commands.

use backbeat_core::AssetId;
use backbeat_sdk::{AssetData, Backbeat, StoreError, assets::AssetDependent};
use base64_simd::STANDARD as BASE64;
use mime_guess::from_path;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppState;

const MAX_ASSET_PREVIEW_SIZE: u64 = 16 * 1024 * 1024;

/// Guess a MIME type from filenames that reference this asset in any bundle.
fn asset_mime_type(store: &Backbeat, asset_id: AssetId) -> backbeat_sdk::Result<Option<String>> {
	Ok(store
		.asset_detail(asset_id)?
		.dependents
		.into_iter()
		.find_map(|dependent| {
			from_path(dependent.path)
				.first()
				.map(|mime| mime.to_string())
		}))
}

fn preview_kind(mime: &str) -> Option<&'static str> {
	if mime.starts_with("image/") {
		Some("image")
	} else if mime.starts_with("audio/") {
		Some("audio")
	} else if mime == "text/plain" {
		Some("text")
	} else {
		None
	}
}

#[derive(Serialize)]
pub struct AssetDetailResponse {
	pub id: AssetId,
	/// `None` if this asset was never (successfully) downloaded by any
	/// chart that references it.
	pub size: Option<u64>,
	/// Whether this asset lives as a file on disk (as opposed to inline in
	/// SQLite, or not stored at all) -- and so can be revealed with
	/// `open_asset_folder`.
	pub has_file: bool,
	pub dependents: Vec<AssetDependent>,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_asset_detail(
	state: State<'_, AppState>,
	asset_id: AssetId,
) -> Result<AssetDetailResponse, String> {
	let store = state.store.get()?;
	let detail = store.asset_detail(asset_id).map_err(|err| {
		tracing::error!(?err, %asset_id, "asset detail query failed");
		err.to_string()
	})?;

	let has_file = matches!(store.get_asset(asset_id), Ok(AssetData::File(_)));

	Ok(AssetDetailResponse {
		id: detail.id,
		size: detail.size,
		has_file,
		dependents: detail.dependents,
	})
}

/// Preview payload for the asset detail screen: MIME guessed from dependent
/// filenames, plus either an on-disk media path or base64-encoded bytes.
#[derive(Serialize)]
pub struct AssetPreviewResponse {
	pub mime_type: String,
	/// `"image"` or `"audio"` when the MIME is something we can display;
	/// otherwise omitted.
	pub kind: Option<String>,
	/// Absolute filesystem path when the asset is stored on disk and previewable.
	pub path: Option<String>,
	/// Base64-encoded bytes for inline media and plain-text previews.
	pub data_base64: Option<String>,
}

/// Sync so Tauri runs this on a blocking thread — opens the asset and may
/// read bytes from SQLite or the filesystem.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_asset_preview(
	state: State<'_, AppState>,
	asset_id: AssetId,
) -> Result<Option<AssetPreviewResponse>, String> {
	let store = state.store.get()?;
	let size = store
		.asset_detail(asset_id)
		.map_err(|err| {
			tracing::error!(?err, %asset_id, "asset detail query failed");
			err.to_string()
		})?
		.size;
	if size.is_some_and(|size| size >= MAX_ASSET_PREVIEW_SIZE) {
		return Ok(None);
	}

	let mime_type = asset_mime_type(store.as_ref(), asset_id).map_err(|err| {
		tracing::error!(?err, %asset_id, "asset mime lookup failed");
		err.to_string()
	})?;

	let Some(mime_type) = mime_type else {
		return Ok(None);
	};

	let Some(kind) = preview_kind(&mime_type) else {
		return Ok(Some(AssetPreviewResponse {
			mime_type,
			kind: None,
			path: None,
			data_base64: None,
		}));
	};

	match store.get_asset(asset_id) {
		Ok(AssetData::File(path)) if kind == "text" => {
			let data = std::fs::read(&path).map_err(|err| err.to_string())?;
			Ok(Some(AssetPreviewResponse {
				mime_type,
				kind: Some(kind.to_string()),
				path: None,
				data_base64: Some(BASE64.encode_to_string(&data)),
			}))
		}
		Ok(AssetData::File(path)) => Ok(Some(AssetPreviewResponse {
			mime_type,
			kind: Some(kind.to_string()),
			path: Some(path.to_string_lossy().into_owned()),
			data_base64: None,
		})),
		Ok(AssetData::Bytes(data)) if kind == "text" => Ok(Some(AssetPreviewResponse {
			mime_type,
			kind: Some(kind.to_string()),
			path: None,
			data_base64: Some(BASE64.encode_to_string(&data)),
		})),
		Ok(AssetData::Bytes(data)) => Ok(Some(AssetPreviewResponse {
			mime_type,
			kind: Some(kind.to_string()),
			path: None,
			data_base64: Some(BASE64.encode_to_string(&data)),
		})),
		Err(StoreError::NotFound(_)) => Ok(Some(AssetPreviewResponse {
			mime_type,
			kind: Some(kind.to_string()),
			path: None,
			data_base64: None,
		})),
		Err(err) => {
			tracing::error!(?err, %asset_id, "failed to open asset for preview");
			Err(err.to_string())
		}
	}
}

/// Reveal the asset's containing folder in the OS's default file manager.
/// Inline assets are first exported to the user's Documents directory.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn open_asset_folder(
	app: AppHandle,
	state: State<'_, AppState>,
	asset_id: AssetId,
) -> Result<(), String> {
	let store = state.store.get()?;
	tokio::task::spawn_blocking(move || {
		let folder = match store.get_asset(asset_id) {
			Ok(AssetData::File(path)) => path
				.parent()
				.ok_or_else(|| "couldn't determine the asset's containing folder".to_string())?
				.to_owned(),
			Ok(AssetData::Bytes(data)) => {
				let filename = store
					.asset_detail(asset_id)
					.map_err(|err| err.to_string())?
					.dependents
					.first()
					.and_then(|dependent| std::path::Path::new(&dependent.path).file_name())
					.map(ToOwned::to_owned)
					.unwrap_or_else(|| asset_id.to_string().into());
				let folder = app
					.path()
					.document_dir()
					.map_err(|err| err.to_string())?
					.join("Backbeat Exports")
					.join("Assets");
				std::fs::create_dir_all(&folder).map_err(|err| err.to_string())?;
				std::fs::write(folder.join(filename), data).map_err(|err| err.to_string())?;
				folder
			}
			Err(err) => return Err(err.to_string()),
		};

		open::that_detached(&folder).map_err(|err| {
			tracing::error!(?err, ?folder, "failed to open asset folder");
			err.to_string()
		})
	})
	.await
	.map_err(|err| err.to_string())?
}
