//! [`SeenCache`]: a path-to-hash index for deduplicating asset work across charts.
//!
//! When importing many charts that share the same keysounds or images (e.g. a
//! BMS pack with dozens of `.bms` files in one directory), the same asset file
//! would otherwise be SHA-256-hashed, read from disk, and stored repeatedly.
//!
//! [`SeenCache`] maps files to their [`AssetId`] (SHA-256). A path in the cache
//! means the asset has already been hashed **and** written to the store in this
//! session. Callers can:
//!
//! - **skip hashing** (in the packager) by looking up the path and reusing the
//!   cached [`AssetId`] directly.
//! - **skip reading and storing** (in the import layer) when the path is already
//!   present, since the asset is guaranteed to be in the store.
//!
//! This relies on the assumption that source files are not mutated while an
//! import session is in progress — canonically identical paths are treated as
//! identical content.
//!
//! Create one [`SeenCache`] per import session and share it across every chart
//! in that session to get the full cross-chart benefit. Because the cache is
//! backed by [`moka::sync::Cache`] it is [`Clone`], [`Send`], and [`Sync`]:
//! a single instance can be shared across threads without external locking.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use backbeat_core::{AssetId, AssetPath};
use dashmap::DashSet;
use moka::sync::Cache;

/// Maximum number of stored assets tracked per session.
const ASSET_CAPACITY: u64 = 10_000;

/// Maximum number of filesystem-existence results memoized per session.
const FILE_EXISTS_CAPACITY: u64 = 100_000;

/// Outcome of [`SeenCache::check_many_exts`].
pub(crate) enum ResolveOutcome {
	/// The asset was found in the cache. No filesystem I/O required.
	Cached {
		/// The actual asset path (after extension substitution).
		actual_path: AssetPath,
		/// The cached asset id; reuse directly.
		asset: AssetId,
	},
	/// The asset exists on disk but has not been cached yet. The caller must
	/// hash it and (via the store layer) add it to the cache.
	NotCached {
		/// Canonical absolute path to the file. Already canonicalized — the
		/// caller can use this directly without another `canonicalize()` call.
		abs: PathBuf,
		/// The actual asset path (after extension substitution).
		actual_path: AssetPath,
	},
}

/// Maps absolute asset paths to their [`AssetId`] for the current import session.
///
/// See the [module documentation](self) for usage details.
///
/// The cache is backed by [`moka::sync::Cache`] and is therefore [`Clone`],
/// [`Send`], and [`Sync`]. Cloning produces a second handle to the **same**
/// underlying cache (Arc semantics), so sharing across threads requires no
/// external synchronisation.
#[derive(Clone)]
pub struct SeenCache {
	/// Assets that have been hashed and stored this session.
	inner: Cache<PathBuf, AssetId>,
	/// Memoized `is_file()` results: `true` = exists, `false` = not found.
	file_exists: Cache<PathBuf, bool>,
	/// Referenced assets that had no matching file on disk.
	///
	/// These are canonicalised paths.
	missing_assets: Arc<DashSet<(PathBuf, AssetPath)>>,
}

impl SeenCache {
	/// Create an empty cache with default capacities.
	pub fn new() -> Self {
		Self {
			inner: Cache::new(ASSET_CAPACITY),
			file_exists: Cache::new(FILE_EXISTS_CAPACITY),
			missing_assets: Arc::new(DashSet::new()),
		}
	}

	/// Resolve `rel_path` under `chart_dir`, trying each extension in `exts`
	/// in order (exact path first, then each substituted extension).
	///
	/// Plenty of rhythm games silently try other extensions if
	/// the one you've provided is invalid.
	///
	/// `is_file()` results are memoized so each path is stat'd at most once
	/// per session.
	///
	/// Returns `None` if no candidate exists on disk.
	pub(crate) fn check_many_exts(
		&self,
		chart_dir: &Path,
		asset_path: &AssetPath,
		exts: &[&str],
	) -> Option<ResolveOutcome> {
		// Try the declared path first, then substitute each extension in order.
		let candidates: Vec<AssetPath> = std::iter::once(asset_path.clone())
			.chain(
				exts.iter()
					.map(|&ext| asset_path.with_extension(ext))
					.collect::<Result<Vec<_>, _>>()
					.expect("extension substitution must preserve an asset path"),
			)
			.collect();

		for actual_path in candidates {
			let abs = chart_dir.join(actual_path.as_path());
			let exists = match self.file_exists.get(&abs) {
				Some(v) => v,
				None => {
					let v = abs.is_file();
					self.file_exists.insert(abs.clone(), v);
					v
				}
			};

			if exists {
				return Some(match self.inner.get(&abs) {
					Some(asset) => ResolveOutcome::Cached { actual_path, asset },
					None => ResolveOutcome::NotCached { abs, actual_path },
				});
			}
		}

		None
	}

	/// Look up `path` in the cache.
	pub(crate) fn get(&self, path: &Path) -> Option<AssetId> {
		self.inner.get(path)
	}

	/// Record `path` as having been hashed and stored with the given `asset`.
	///
	/// After calling this, [`get`] will return `Some` for the same path.
	///
	/// [`get`]: Self::get
	pub(crate) fn insert(&self, path: &Path, asset: AssetId) {
		self.inner.insert(path.to_owned(), asset);
	}

	pub(crate) fn record_missing(&self, chart_dir: &Path, rel_path: &AssetPath) {
		self.missing_assets
			.insert((chart_dir.to_owned(), rel_path.to_owned()));
	}

	/// Return the referenced asset paths that had no matching file on disk.
	pub fn missing_asset_paths(&self) -> Vec<AssetPath> {
		self.missing_assets.iter().map(|e| e.1.clone()).collect()
	}

	/// Create a cache view that shares resolved assets and filesystem lookups,
	/// but collects missing references independently.
	pub fn with_isolated_missing_assets(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			file_exists: self.file_exists.clone(),
			missing_assets: Arc::new(DashSet::new()),
		}
	}
}

impl Default for SeenCache {
	fn default() -> Self {
		Self::new()
	}
}
