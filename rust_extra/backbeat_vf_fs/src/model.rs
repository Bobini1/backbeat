//! Backend-agnostic filesystem state: path index, store reads, and inode metadata.
//!
//! The [`fuser`] glue in [`crate::fs`] delegates here so a future WinFsp backend
//! can reuse the same logic.

use std::io;
use std::sync::Arc;
use std::time::SystemTime;

use backbeat_core::AssetId;
use backbeat_core::{BundleId, ChartId, IdAlgorithm, Sha256};
use backbeat_sdk::AssetData;
use backbeat_sdk::Backbeat;
use moka::notification::RemovalCause;
use moka::sync::Cache;
use parking_lot::Mutex;

use crate::dentry_backend::DentryIndex;
use crate::index::{NodeKind, ROOT_INODE};
use crate::{Attrs, DirEntry, FileKind};

/// Resolved asset payload for repeated reads on the same open handle.
#[derive(Debug, Clone)]
pub(crate) enum AssetBody {
	Inline(Arc<Vec<u8>>),
	File(Arc<Mutex<std::fs::File>>),
}

/// Open file / directory handle state.
#[derive(Debug, Clone)]
enum Handle {
	Chart {
		source: ChartSource,
	},
	Asset {
		body: AssetBody,
	},
	Dir {
		#[allow(dead_code)]
		inode: u64,
	},
}

#[derive(Debug)]
struct HandleTable {
	slots: Vec<Option<Handle>>,
	free: Vec<usize>,
}

impl HandleTable {
	fn new() -> Self {
		Self {
			slots: vec![None],
			free: Vec::new(),
		}
	}

	fn insert(&mut self, handle: Handle) -> u64 {
		let index = self.free.pop().unwrap_or_else(|| {
			self.slots.push(None);
			self.slots.len() - 1
		});
		self.slots[index] = Some(handle);
		index as u64
	}

	fn get(&self, fh: u64) -> Option<Handle> {
		let index = usize::try_from(fh).ok()?;
		self.slots.get(index)?.clone()
	}

	fn remove(&mut self, fh: u64) {
		let Ok(index) = usize::try_from(fh) else {
			return;
		};
		let Some(slot) = self.slots.get_mut(index) else {
			return;
		};
		if slot.take().is_some() {
			self.free.push(index);
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ChartSource {
	Stored(Sha256),
	Melded(Box<[BundleId]>),
}

/// Read-only Backbeat export filesystem state.
pub struct BackbeatFilesystem {
	index: Arc<dyn DentryIndex>,
	store: Backbeat,
	chart_bytes: Cache<ChartSource, Arc<Vec<u8>>>,
	handles: Mutex<HandleTable>,
}

impl BackbeatFilesystem {
	pub fn new(index: Arc<dyn DentryIndex>, store: Backbeat) -> Self {
		Self {
			index,
			store,
			chart_bytes: Self::build_chart_cache(1024 * 1024 * 1024),
			handles: Mutex::new(HandleTable::new()),
		}
	}

	fn build_chart_cache(max_bytes: u64) -> Cache<ChartSource, Arc<Vec<u8>>> {
		Cache::builder()
			.name("backbeat-chart-bytes")
			.max_capacity(max_bytes)
			.weigher(|_, bytes: &Arc<Vec<u8>>| u32::try_from(bytes.capacity()).unwrap_or(u32::MAX))
			.eviction_listener(move |source, bytes, cause| {
				let chart_bytes = bytes.len() as u64;
				let allocated_bytes = bytes.capacity() as u64;
				if cause == RemovalCause::Size && max_bytes > 0 && allocated_bytes > max_bytes {
					tracing::warn!(
						?source,
						chart_bytes,
						allocated_bytes,
						chart_cache_budget = max_bytes,
						"rendered chart exceeds cache budget and was not retained"
					);
				} else {
					tracing::debug!(
						?source,
						?cause,
						chart_bytes,
						allocated_bytes,
						chart_cache_budget = max_bytes,
						"removed rendered chart from virtual-filesystem cache"
					);
				}
			})
			.build()
	}

	pub fn index(&self) -> &dyn DentryIndex {
		self.index.as_ref()
	}

	pub fn lookup(&self, parent: u64, name: &str) -> Result<Attrs, io::Error> {
		let child_inode = self
			.index
			.lookup_child(parent, name)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		self.node_attr(child_inode)
	}

	pub fn getattr(&self, inode: u64) -> Result<Attrs, io::Error> {
		self.node_attr(inode)
	}

	pub fn open(&self, inode: u64) -> Result<u64, io::Error> {
		let node = self
			.index
			.get(inode)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

		let handle = match &node.kind {
			NodeKind::Chart { sha256, .. } => Handle::Chart {
				source: ChartSource::Stored(*sha256),
			},
			NodeKind::MeldedChart { bundle_ids } => Handle::Chart {
				source: ChartSource::Melded(bundle_ids.clone()),
			},
			NodeKind::Asset { asset_id, .. } => Handle::Asset {
				body: self.resolve_asset(*asset_id)?,
			},
			NodeKind::Directory => return Err(io::Error::from_raw_os_error(libc::EISDIR)),
		};

		Ok(self.alloc_fh(handle))
	}

	pub fn opendir(&self, inode: u64) -> Result<u64, io::Error> {
		let node = self
			.index
			.get(inode)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		if !matches!(node.kind, NodeKind::Directory) {
			return Err(io::Error::from_raw_os_error(libc::ENOTDIR));
		}
		Ok(self.alloc_fh(Handle::Dir { inode }))
	}

	pub fn read(&self, fh: u64, offset: u64, size: u32) -> Result<Vec<u8>, io::Error> {
		let handle = self
			.get_handle(fh)
			.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
		self.read_handle(&handle, offset, size)
	}

	pub fn readdir_entries(&self, dir_inode: u64, offset: u64) -> Result<Vec<DirEntry>, io::Error> {
		let node = self
			.index
			.get(dir_inode)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		if !matches!(node.kind, NodeKind::Directory) {
			return Err(io::Error::from_raw_os_error(libc::ENOTDIR));
		}

		let mut all: Vec<(u64, FileKind, String)> = vec![
			(dir_inode, FileKind::Directory, ".".to_owned()),
			(
				node.parent.unwrap_or(ROOT_INODE),
				FileKind::Directory,
				"..".to_owned(),
			),
		];

		if let Some(children) = self.index.list_children(dir_inode) {
			for (name, child_inode, kind) in children {
				all.push((child_inode, node_file_type(&kind), name));
			}
		}

		Ok(all
			.into_iter()
			.enumerate()
			.filter(|(idx, _)| (*idx as u64 + 1) > offset)
			.map(|(idx, (inode, kind, name))| DirEntry {
				ino: inode,
				kind,
				name,
				next: (idx + 1) as u64,
			})
			.collect())
	}

	pub fn release(&self, fh: u64) {
		self.handles.lock().remove(fh);
	}

	/// Backing path for a file-backed asset inode, if the asset lives on disk.
	pub fn asset_backing_path(&self, inode: u64) -> Result<Option<std::path::PathBuf>, io::Error> {
		let node = self
			.index
			.get(inode)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

		let NodeKind::Asset { asset_id, .. } = node.kind else {
			return Ok(None);
		};

		Ok(
			match self
				.store
				.get_asset(asset_id)
				.map_err(|e| e.into_io_error())?
			{
				AssetData::File(path) => Some(path),
				AssetData::Bytes(_) => None,
			},
		)
	}

	/// Server-side `copy_file_range(2)` between two open file-backed asset handles.
	#[cfg(target_os = "linux")]
	pub fn copy_file_range_between_handles(
		&self,
		fh_in: u64,
		offset_in: u64,
		fh_out: u64,
		offset_out: u64,
		len: u64,
	) -> Result<u32, io::Error> {
		use std::io::{Read, Seek, SeekFrom, copy};

		let handle_in = self
			.get_handle(fh_in)
			.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
		let handle_out = self
			.get_handle(fh_out)
			.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;

		let (
			Handle::Asset {
				body: AssetBody::File(src),
			},
			Handle::Asset {
				body: AssetBody::File(dest),
			},
		) = (handle_in, handle_out)
		else {
			return Err(io::Error::from_raw_os_error(libc::EXDEV));
		};

		let mut src = src.lock();
		let mut dest = dest.lock();
		src.seek(SeekFrom::Start(offset_in))?;
		dest.seek(SeekFrom::Start(offset_out))?;
		let copied = copy(&mut (&mut *src).take(len), &mut *dest)?;
		Ok(copied as u32)
	}

	fn node_attr(&self, inode: u64) -> Result<Attrs, io::Error> {
		let node = self
			.index
			.get(inode)
			.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

		let now = SystemTime::now();
		let (kind, size) = match &node.kind {
			NodeKind::Directory => (FileKind::Directory, 0),
			NodeKind::Chart { .. } => (FileKind::RegularFile, node.size),
			NodeKind::MeldedChart { bundle_ids } => (
				FileKind::RegularFile,
				self.load_chart_bytes(ChartSource::Melded(bundle_ids.clone()))?
					.len() as u64,
			),
			NodeKind::Asset { size, .. } => (FileKind::RegularFile, *size),
		};

		let nlink = if matches!(node.kind, NodeKind::Directory) {
			2
		} else {
			1
		};

		Ok(Attrs {
			ino: inode,
			size,
			blocks: size.div_ceil(512),
			atime: now,
			mtime: now,
			ctime: now,
			crtime: now,
			kind,
			perm: if matches!(node.kind, NodeKind::Directory) {
				0o555
			} else {
				0o444
			},
			nlink,
			uid: None,
			gid: None,
			rdev: 0,
			blksize: 4096,
		})
	}

	fn resolve_asset(&self, asset_id: AssetId) -> Result<AssetBody, io::Error> {
		match self
			.store
			.get_asset(asset_id)
			.map_err(|e| e.into_io_error())?
		{
			AssetData::Bytes(bytes) => Ok(AssetBody::Inline(Arc::new(bytes))),
			AssetData::File(file_path) => {
				let file = std::fs::File::open(&file_path)?;
				Ok(AssetBody::File(Arc::new(Mutex::new(file))))
			}
		}
	}

	fn read_handle(&self, handle: &Handle, offset: u64, size: u32) -> Result<Vec<u8>, io::Error> {
		match handle {
			Handle::Chart { source } => {
				let bytes = self.load_chart_bytes(source.clone())?;
				let start = offset.min(bytes.len() as u64) as usize;
				let end = offset.saturating_add(size as u64).min(bytes.len() as u64) as usize;
				Ok(bytes[start..end].to_vec())
			}
			Handle::Asset { body } => match body {
				AssetBody::Inline(bytes) => {
					let start = offset.min(bytes.len() as u64) as usize;
					let end = offset.saturating_add(size as u64).min(bytes.len() as u64) as usize;
					Ok(bytes[start..end].to_vec())
				}
				AssetBody::File(file) => {
					use std::io::{Read as _, Seek as _, SeekFrom};
					let mut file = file.lock();
					file.seek(SeekFrom::Start(offset))?;
					let mut buf = vec![0u8; size as usize];
					let n = file.read(&mut buf)?;
					buf.truncate(n);
					Ok(buf)
				}
			},
			Handle::Dir { .. } => Err(io::Error::from_raw_os_error(libc::EISDIR)),
		}
	}

	fn load_chart_bytes(&self, source: ChartSource) -> Result<Arc<Vec<u8>>, io::Error> {
		self.chart_bytes
			.try_get_with(source.clone(), || {
				self.load_chart_bytes_uncached(&source).map(Arc::new)
			})
			.map_err(|err| {
				tracing::error!(?source, error = %err, "failed to render chart for virtual filesystem");
				io::Error::other(err.to_string())
			})
	}

	fn load_chart_bytes_uncached(&self, source: &ChartSource) -> Result<Vec<u8>, io::Error> {
		match source {
			ChartSource::Stored(sha256) => {
				let chart_id = ChartId {
					alg: IdAlgorithm::Sha256,
					val: sha256.to_string(),
				};
				self.store
					.get_chart_data(&chart_id)
					.map_err(io::Error::other)
			}
			ChartSource::Melded(bundle_ids) => {
				let bundles = bundle_ids
					.iter()
					.map(|bundle_id| self.store.get_bundle(*bundle_id))
					.collect::<Result<Vec<_>, _>>()
					.map_err(io::Error::other)?;
				backbeat_packager::meld_chart_bytes(&bundles)
					.map(|bytes| bytes.into_vec())
					.map_err(io::Error::other)
			}
		}
	}

	fn alloc_fh(&self, handle: Handle) -> u64 {
		self.handles.lock().insert(handle)
	}

	fn get_handle(&self, fh: u64) -> Option<Handle> {
		self.handles.lock().get(fh)
	}
}

fn node_file_type(kind: &NodeKind) -> FileKind {
	match kind {
		NodeKind::Directory => FileKind::Directory,
		NodeKind::Chart { .. } | NodeKind::MeldedChart { .. } | NodeKind::Asset { .. } => {
			FileKind::RegularFile
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::thread;
	use std::time::Duration;

	use backbeat_core::Sha256;

	use super::{BackbeatFilesystem, ChartSource, Handle, HandleTable};

	fn chart_source(value: u8) -> ChartSource {
		ChartSource::Stored(Sha256::from([value; 32]))
	}

	fn chart_bytes(size: usize) -> Arc<Vec<u8>> {
		Arc::new(vec![0; size])
	}

	#[test]
	fn released_handle_slots_are_reused_once() {
		let mut handles = HandleTable::new();
		let first = handles.insert(Handle::Dir { inode: 1 });
		let second = handles.insert(Handle::Dir { inode: 2 });

		handles.remove(first);
		handles.remove(first);

		let replacement = handles.insert(Handle::Dir { inode: 3 });
		let next = handles.insert(Handle::Dir { inode: 4 });

		assert_eq!(replacement, first);
		assert_ne!(next, replacement);
		assert!(handles.get(second).is_some());
	}

	#[test]
	fn chart_cache_evicts_by_allocated_bytes() {
		let cache = BackbeatFilesystem::build_chart_cache(10);
		cache.insert(chart_source(1), chart_bytes(4));
		cache.insert(chart_source(2), chart_bytes(6));
		cache.run_pending_tasks();

		assert_eq!(cache.entry_count(), 2);
		assert_eq!(cache.weighted_size(), 10);

		cache.insert(chart_source(3), chart_bytes(5));
		cache.run_pending_tasks();

		assert!(cache.entry_count() < 3);
		assert!(cache.weighted_size() <= 10);
	}

	#[test]
	fn chart_cache_does_not_retain_oversized_values() {
		let cache = BackbeatFilesystem::build_chart_cache(10);
		let source = chart_source(1);
		cache.insert(source.clone(), chart_bytes(11));
		cache.run_pending_tasks();

		assert!(!cache.contains_key(&source));
		assert_eq!(cache.weighted_size(), 0);
	}

	#[test]
	fn concurrent_oversized_chart_loads_are_coalesced_without_retention() {
		const READERS: usize = 8;

		let cache = Arc::new(BackbeatFilesystem::build_chart_cache(10));
		let source = chart_source(1);
		let ready = Arc::new(AtomicUsize::new(0));
		let initializations = Arc::new(AtomicUsize::new(0));
		let mut readers = Vec::new();

		for _ in 0..READERS {
			let cache = Arc::clone(&cache);
			let source = source.clone();
			let ready = Arc::clone(&ready);
			let initializations = Arc::clone(&initializations);
			readers.push(thread::spawn(move || {
				ready.fetch_add(1, Ordering::SeqCst);
				cache
					.try_get_with(source, || {
						initializations.fetch_add(1, Ordering::SeqCst);
						while ready.load(Ordering::SeqCst) < READERS {
							thread::yield_now();
						}
						thread::sleep(Duration::from_millis(25));
						Ok::<_, ()>(chart_bytes(11))
					})
					.unwrap()
			}));
		}

		let values = readers
			.into_iter()
			.map(|reader| reader.join().unwrap())
			.collect::<Vec<_>>();
		cache.run_pending_tasks();

		assert_eq!(initializations.load(Ordering::SeqCst), 1);
		assert!(
			values
				.windows(2)
				.all(|pair| Arc::ptr_eq(&pair[0], &pair[1]))
		);
		assert!(!cache.contains_key(&source));
		assert_eq!(cache.weighted_size(), 0);
	}
}
