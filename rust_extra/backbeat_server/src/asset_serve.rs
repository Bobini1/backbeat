use std::path::PathBuf;

use backbeat_core::AssetId;
use backbeat_sdk::AssetData;
use backbeat_sdk::{Backbeat, Result};
use mime_guess::from_path;

/// How an asset should be delivered over HTTP.
#[derive(Debug, Clone)]
pub enum AssetServeResponse {
	/// Small inline asset bytes from SQLite.
	Bytes {
		data: Vec<u8>,
		content_type: Option<String>,
		len: u64,
	},
	/// Large asset streamed from the local filesystem.
	File {
		path: PathBuf,
		content_type: Option<String>,
		len: u64,
	},
}

/// Resolve how to serve an asset over HTTP.
pub fn serve_asset(store: &Backbeat, asset_id: AssetId) -> Result<AssetServeResponse> {
	let content_type = asset_mime_type(store, asset_id)?;

	match store.get_asset(asset_id)? {
		AssetData::Bytes(data) => {
			let len = data.len() as u64;
			Ok(AssetServeResponse::Bytes {
				data,
				content_type,
				len,
			})
		}
		AssetData::File(path) => {
			let len = std::fs::metadata(&path)?.len();
			Ok(AssetServeResponse::File {
				path,
				content_type,
				len,
			})
		}
	}
}

/// Guess a MIME type from filenames that reference this asset in any bundle.
pub fn asset_mime_type(store: &Backbeat, asset_id: AssetId) -> Result<Option<String>> {
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
