use std::collections::HashSet;
use std::path::Path;

use backbeat_core::AssetId;

use crate::Result;
use crate::store::Backbeat;

/// The result of doing disk cleans.
pub struct PruneReport {
	/// DB orphan assets removed (or would be removed in dry-run).
	pub removed_db_assets: usize,
	/// Bytes freed by removing large DB orphans from disk.
	pub freed_db_asset_bytes: u64,
	/// Disk orphan files removed (or would be removed in dry-run).
	pub removed_disk_orphans: usize,
	/// Bytes freed by removing disk orphans.
	pub freed_disk_orphan_bytes: u64,
	/// When true, nothing was actually deleted.
	pub dry_run: bool,
	/// When true, the disk orphan treewalk was performed.
	pub full: bool,
}

impl PruneReport {
	/// Total bytes that were (or would be) freed across both prune steps.
	pub fn total_freed_bytes(&self) -> u64 {
		self.freed_db_asset_bytes + self.freed_disk_orphan_bytes
	}

	/// `true` if nothing would be or was removed.
	pub fn is_empty(&self) -> bool {
		self.removed_db_assets == 0 && self.removed_disk_orphans == 0
	}
}

impl Backbeat {
	pub(crate) fn disk_prune_assets(
		root: &Path,
		dir: &Path,
		known: &HashSet<AssetId>,
		dry_run: bool,
	) -> Result<u64> {
		let mut removed = 0;
		for entry in crate::fs::read_dir(dir)? {
			let entry = entry?;
			let file_type = entry.file_type()?;
			if file_type.is_dir() {
				removed += Self::disk_prune_assets(root, &entry.path(), known, dry_run)?;
				continue;
			}
			if !file_type.is_file() {
				continue;
			}

			let relative = entry
				.path()
				.strip_prefix(root)
				.expect("directory entries are beneath their root")
				.to_path_buf();
			let Some(asset_id) = AssetId::from_fanned_path(relative) else {
				continue;
			};
			if known.contains(&asset_id) {
				continue;
			}

			removed += 1;
			if !dry_run {
				crate::fs::remove_file(entry.path())?;
			}
		}
		Ok(removed)
	}
}
