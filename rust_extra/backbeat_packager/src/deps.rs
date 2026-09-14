//! Shared dependency resolution: ensure referenced files are recorded in `assets`,
//! hashing them from disk when not already scanned.

use std::path::{Component, Path};

use backbeat_core::{AssetId, AssetPath, BackbeatFile, Sha256};

use crate::error::FromFileError;
use crate::seen_cache::{ResolveOutcome, SeenCache};

// todo: move this to Assets
pub(crate) fn assert_not_pathing_up_too_much(
	bb: &BackbeatFile,
	max_depth: usize,
) -> Result<(), FromFileError> {
	for path in bb.assets.keys() {
		if max_parent_depth(Path::new(path.as_str())) > max_depth {
			return Err(FromFileError::AssetPathTooDeep {
				path: path.to_string(),
				max_depth,
			});
		}
	}
	Ok(())
}

fn max_parent_depth(path: &Path) -> usize {
	let mut depth = 0isize;
	let mut max_parent_depth = 0usize;
	for component in path.components() {
		match component {
			Component::CurDir => {}
			Component::ParentDir => {
				depth -= 1;
				max_parent_depth = max_parent_depth.max((-depth).max(0) as usize);
			}
			Component::Normal(_) => depth += 1,
			Component::RootDir | Component::Prefix(_) => {}
		}
	}
	max_parent_depth
}

/// Ensure a referenced asset is recorded in `assets`.
///
/// Resolves `unicode_path_bytes` under `chart_dir`, optionally checking the list of
/// extra_exts if non-empty
/// Missing files are skipped.
pub(crate) fn ensure_asset(
	bb: &mut BackbeatFile,
	chart_dir: &Path,
	cache: &SeenCache,
	unicode_path_bytes: &str,
	extra_exts: &[&str],
) -> Result<(), FromFileError> {
	if unicode_path_bytes.is_empty() {
		return Ok(());
	}

	let asset_path = AssetPath::from_path(unicode_path_bytes)?;

	let Some(outcome) = cache.check_many_exts(chart_dir, &asset_path, extra_exts) else {
		cache.record_missing(chart_dir, &asset_path);
		return Ok(());
	};

	let (asset_path, asset_id) = match outcome {
		ResolveOutcome::Cached { actual_path, asset } => (actual_path, asset),
		ResolveOutcome::NotCached { abs, actual_path } => {
			let sha256 = Sha256::open_and_checksum(&abs)?;
			let id = AssetId(sha256);
			cache.insert(&abs, id);
			(actual_path, id)
		}
	};

	bb.assets.insert(asset_path, asset_id);

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::format::Format;

	#[test]
	fn max_parent_depth_tracks_the_furthest_upward_reference() {
		assert_eq!(max_parent_depth(Path::new("song.ogg")), 0);
		assert_eq!(max_parent_depth(Path::new("../banner.png")), 1);
		assert_eq!(max_parent_depth(Path::new("../../shared/banner.png")), 2);
		assert_eq!(max_parent_depth(Path::new("assets/../../banner.png")), 1);
	}

	#[test]
	fn max_allowed_up_pathing_is_explicit_for_every_extension() {
		assert_eq!(Format::Bms.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Bme.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Bml.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Pms.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Sm.max_allowed_up_pathing(), 1);
		assert_eq!(Format::Ssc.max_allowed_up_pathing(), 1);
		assert_eq!(Format::Dwi.max_allowed_up_pathing(), 1);
		assert_eq!(Format::Bmson.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Ksh.max_allowed_up_pathing(), 5);
		assert_eq!(Format::Kson.max_allowed_up_pathing(), 5);
	}
}
