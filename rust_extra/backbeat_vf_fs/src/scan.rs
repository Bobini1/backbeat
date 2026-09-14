//! Build a [`PathIndex`] from a store's [`PathTreeSnapshot`].

use backbeat_core::asset_id::AssetId;
use backbeat_sdk::Backbeat;

use crate::index::{IndexError, PathIndex};

/// Build the in-memory path index from a live store snapshot for `source`
/// (a virtual folder url).
pub fn build_index(store: &Backbeat, source: &str) -> Result<PathIndex, IndexError> {
	let snapshot = store
		.path_tree_snapshot(source)
		.map_err(|err| IndexError::InvalidPath {
			path: String::new(),
			reason: err.to_string(),
		})?;
	build_index_from_snapshot(&snapshot)
}

/// Build the in-memory path index from a preloaded snapshot. [`build_index`]
/// is the production entry point; this is exposed separately so tests can
/// construct a [`PathTreeSnapshot`] by hand instead of round-tripping
/// through a live store.
pub fn build_index_from_snapshot(snapshot: &PathTreeSnapshot) -> Result<PathIndex, IndexError> {
	let mut index = PathIndex::new();

	for row in &snapshot.entries {
		match row.kind {
			DentryKind::Chart => {
				index.insert_chart(&row.path, row.sha256, row.size)?;
			}
			DentryKind::Asset => {
				index.insert_asset(&row.path, AssetId(row.sha256), row.size)?;
			}
			DentryKind::Directory => {
				// A snapshot only ever contains leaf rows; a directory kind
				// here would mean the catalog produced a non-leaf, which is a
				// bug rather than something to silently skip.
				return Err(IndexError::InvalidPath {
					path: row.path.clone(),
					reason: "unexpected directory entry in path tree snapshot".into(),
				});
			}
		}
	}

	Ok(index)
}

#[cfg(test)]
mod tests {
	use super::*;
	use backbeat_core::Sha256;
	use backbeat_sdk::PathEntryRow;

	#[test]
	fn builds_tree_from_flat_entries() {
		let chart_sha = Sha256::null();
		let asset_sha: Sha256 = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7"
			.parse()
			.unwrap();

		let snapshot = PathTreeSnapshot {
			url: "https://data.backbeat.ac/virtual-folders/stepmania".into(),
			entries: vec![
				PathEntryRow {
					path: "group/song.bms".into(),
					kind: DentryKind::Chart,
					sha256: chart_sha,
					size: 0,
				},
				PathEntryRow {
					path: "group/sound/kick.ogg".into(),
					kind: DentryKind::Asset,
					sha256: asset_sha,
					size: 42,
				},
			],
		};

		let index = build_index_from_snapshot(&snapshot).expect("build index");
		assert!(index.lookup_path("group/song.bms").is_some());
		assert!(index.lookup_path("group/sound/kick.ogg").is_some());
	}
}
