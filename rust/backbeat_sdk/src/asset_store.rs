//! Content-addressed asset storage on disk.
//!
//! Large assets are stored using a fanned SHA-256 path layout that mirrors Git's
//! object store:
//!
//! ```text
//! assets/
//!   12/34/56restofhash
//!   ab/cd/efrestofhash
//! ```
//!
//! This is an internal implementation detail of [`Backbeat`]. Small assets
//! (<= [`SMALL_ASSET_THRESHOLD`]) are stored in SQLite instead; this type only
//! manages the on-disk portion.
//!
//! [`Backbeat`]: crate::store::Backbeat
//! [`SMALL_ASSET_THRESHOLD`]: crate::store::SMALL_ASSET_THRESHOLD

use std::io;
use std::path::{Path, PathBuf};

use backbeat_core::asset_id::AssetId;

/// A content-addressed store for Backbeat assets on disk.
#[derive(Clone)]
pub(crate) struct AssetStore {
	root: PathBuf,
}

impl AssetStore {
	/// Open (or create) an asset store rooted at `root`.
	pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
		Self { root: root.into() }
	}

	/// The path where `asset` would be stored (file may not yet exist).
	pub(crate) fn asset_path(&self, asset: AssetId) -> PathBuf {
		self.root.join(asset.fanned_path())
	}

	/// Write `data` into the store for `asset`.
	///
	/// No-op if the asset is already present. Parent directories are created
	/// as needed.
	pub(crate) fn store(&self, asset: AssetId, data: &[u8]) -> io::Result<()> {
		let path = self.asset_path(asset);

		if path.is_file() {
			return Ok(());
		}

		if let Some(parent) = path.parent() {
			crate::fs::create_dir_all(parent)?;
		}

		crate::fs::write(&path, data)?;
		Ok(())
	}

	/// Directly copy this path into the right place in the
	/// asset store, no in-memory stuff.
	pub(crate) fn copy_into(&self, asset: AssetId, path: impl AsRef<Path>) -> io::Result<()> {
		let dest = self.asset_path(asset);

		if dest.is_file() {
			return Ok(());
		}

		if let Some(parent) = dest.parent() {
			crate::fs::create_dir_all(parent)?;
		}

		fs_err::copy(path, dest)?;
		Ok(())
	}
}
