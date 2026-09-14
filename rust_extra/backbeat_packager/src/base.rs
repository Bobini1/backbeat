//! Format-agnostic base step: read chart bytes, compress, scan sibling assets.

use std::collections::HashMap;
use std::path::Path;

use backbeat_core::AssetPath;
use backbeat_core::{
	AssetId, Assets, BackbeatFile, ChartData, ChartDesc, ChartFilename, ChartFilenameError, Sha256,
};
use walkdir::WalkDir;

use crate::blacklist::should_ignore_file;
use crate::error::FromFileError;
use crate::seen_cache::SeenCache;

/// Extract the chart filename for storage in [`BackbeatFile::filename`].
pub(crate) fn chart_filename(chart_path: &Path) -> Result<ChartFilename, ChartFilenameError> {
	let name = chart_path.file_name().ok_or(ChartFilenameError::Empty)?;
	let name = name.to_str().ok_or(ChartFilenameError::NotUnicode)?;

	ChartFilename::from_path(name)
}

/// Build a [`BackbeatFile`] from any on-disk file: compress chart bytes, scan the
/// chart directory into `assets`, set `filename`.
pub(crate) fn from_file(
	path: impl AsRef<Path>,
	cache: &SeenCache,
) -> Result<BackbeatFile, FromFileError> {
	let path = path.as_ref();
	let bytes = std::fs::read(path)?;
	from_bytes(&bytes, path, cache)
}

/// Like [`from_file`] but accepts chart bytes already in memory.
pub(crate) fn from_bytes(
	bytes: &[u8],
	chart_path: &Path,
	cache: &SeenCache,
) -> Result<BackbeatFile, FromFileError> {
	let chart_dir = chart_path.parent().unwrap_or(Path::new("."));
	let filename = chart_filename(chart_path)?;

	Ok(BackbeatFile {
		filename,
		assets: scan_folder_assets(chart_dir, cache)?,
		desc: ChartDesc::new("").expect("empty chart description is valid"),
		chart: ChartData::compress(bytes)?,
	})
}

/// Hash every non-blacklisted file under `dir`, inserting results into `cache`.
///
/// Returns a map of `relative/path/to/file` → [`AssetId`].
pub(crate) fn scan_folder_assets(dir: &Path, cache: &SeenCache) -> Result<Assets, FromFileError> {
	let mut assets = HashMap::new();

	for entry in WalkDir::new(dir).follow_links(false).into_iter().flatten() {
		if !entry.file_type().is_file() {
			continue;
		}

		// ignore all non-unicode paths in a dir
		if entry.path().to_str().is_none() {
			return Err(FromFileError::AssetPath(
				backbeat_core::AssetPathError::NotUnicode,
			));
		}

		let abs = entry.path();

		let Some(filename) = abs.file_name().and_then(|n| n.to_str()) else {
			continue;
		};
		if should_ignore_file(filename) {
			continue;
		}

		let Ok(rel) = abs.strip_prefix(dir) else {
			continue;
		};

		let asset_id = if let Some(cached) = cache.get(abs) {
			cached
		} else {
			let Ok(sha256) = Sha256::open_and_checksum(abs) else {
				continue;
			};
			let id = AssetId(sha256);
			cache.insert(abs, id);
			id
		};

		let asset_path = AssetPath::from_path(rel)?;

		assets.insert(asset_path, asset_id);
	}

	Ok(assets)
}
