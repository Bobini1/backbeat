//! Abstraction over the in-memory path index used to serve FUSE/WinFsp
//! lookups.

use backbeat_sdk::Backbeat;

use crate::index::{IndexError, NodeKind, PathIndex, ROOT_INODE};
// use crate::scan::build_index;

/// Read-only path index used by [`crate::model::BackbeatFilesystem`].
pub trait DentryIndex: Send + Sync {
	fn get(&self, inode: u64) -> Option<DentryNode>;
	fn lookup_child(&self, parent: u64, name: &str) -> Option<u64>;
	fn list_children(&self, dir_inode: u64) -> Option<Vec<(String, u64, NodeKind)>>;
	fn len(&self) -> u64;
	fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

/// Normalised inode metadata for FUSE operations.
#[derive(Debug, Clone)]
pub struct DentryNode {
	pub inode: u64,
	pub kind: NodeKind,
	pub parent: Option<u64>,
	pub size: u64,
}

/// In-memory index backed by a [`PathIndex`].
pub struct MemDentryIndex {
	index: PathIndex,
}

impl MemDentryIndex {
	pub fn new(index: PathIndex) -> Self {
		Self { index }
	}
}

impl DentryIndex for MemDentryIndex {
	fn get(&self, inode: u64) -> Option<DentryNode> {
		let node = self.index.get(inode)?;
		let size = match &node.kind {
			NodeKind::Chart { size, .. } | NodeKind::Asset { size, .. } => *size,
			NodeKind::MeldedChart { .. } => 0,
			NodeKind::Directory => 0,
		};
		Some(DentryNode {
			inode: node.inode,
			kind: node.kind.clone(),
			parent: node.parent,
			size,
		})
	}

	fn lookup_child(&self, parent: u64, name: &str) -> Option<u64> {
		let parent_node = self.index.get(parent)?;
		if !matches!(parent_node.kind, NodeKind::Directory) {
			return None;
		}
		match name {
			"." => Some(parent),
			".." => Some(parent_node.parent.unwrap_or(ROOT_INODE)),
			other => parent_node.children.get(other).copied(),
		}
	}

	fn list_children(&self, dir_inode: u64) -> Option<Vec<(String, u64, NodeKind)>> {
		let node = self.index.get(dir_inode)?;
		if !matches!(node.kind, NodeKind::Directory) {
			return None;
		}
		Some(
			self.index
				.child_names(dir_inode)?
				.into_iter()
				.map(|(name, inode)| {
					let kind = self
						.index
						.get(inode)
						.map(|n| n.kind.clone())
						.unwrap_or(NodeKind::Directory);
					(name, inode, kind)
				})
				.collect(),
		)
	}

	fn len(&self) -> u64 {
		self.index.len() as u64
	}
}

/// Build the full in-memory dentry index for virtual folder `source`.
pub fn open_index(_store: &Backbeat, _source: &str) -> Result<MemDentryIndex, IndexError> {
	todo!("unimplemented")
	// let index = build_index(store, source)?;
	// Ok(MemDentryIndex::new(index))
}
