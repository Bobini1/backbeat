//! Logical export-path index: an in-memory tree of a virtual folder's
//! mount-relative paths, built from a [`backbeat_sdk::PathTreeSnapshot`].

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use backbeat_core::asset_id::AssetId;
use backbeat_core::{BundleId, Sha256};

/// FUSE root inode (conventional).
pub const ROOT_INODE: u64 = 1;

/// What a leaf or directory node represents in the export tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
	/// Directory with children.
	Directory,
	/// Chart file at its full `path`.
	Chart { sha256: Sha256, size: u64 },
	/// Chart file rendered by melding the listed source bundles.
	MeldedChart { bundle_ids: Box<[BundleId]> },
	/// Asset file at `parent(path) / filename`.
	Asset { asset_id: AssetId, size: u64 },
}

/// Metadata for one inode in the export tree.
#[derive(Debug, Clone)]
pub struct Node {
	pub inode: u64,
	pub kind: NodeKind,
	/// Logical path from mount root, using `/` separators. Empty for root.
	pub path: String,
	pub parent: Option<u64>,
	pub children: BTreeMap<String, u64>,
}

/// In-memory index of all export paths and assets.
#[derive(Debug, Default)]
pub struct PathIndex {
	next_inode: u64,
	nodes: BTreeMap<u64, Node>,
}

/// Errors while constructing a [`PathIndex`].
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),

	#[error("corrupt database: invalid {field} {value:?}")]
	InvalidValue { field: &'static str, value: String },

	#[error("path collision at {path:?}: existing {existing:?}, new {new_kind:?}")]
	PathCollision {
		path: String,
		existing: NodeKind,
		new_kind: NodeKind,
	},

	#[error(
		"asset conflict at {path:?}: charts disagree on content (expected {expected:?}, found {found:?})"
	)]
	AssetConflict {
		path: String,
		expected: AssetId,
		found: AssetId,
	},

	#[error("invalid export path {path:?}: {reason}")]
	InvalidPath { path: String, reason: String },

	#[error("chart collision at {path:?}: {filename} does not support melding")]
	UnmeldableChartCollision { path: String, filename: String },
}

impl PartialEq for IndexError {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Database(a), Self::Database(b)) => a.to_string() == b.to_string(),
			(
				Self::InvalidValue {
					field: field_a,
					value: value_a,
				},
				Self::InvalidValue {
					field: field_b,
					value: value_b,
				},
			) => field_a == field_b && value_a == value_b,
			(
				Self::PathCollision {
					path: path_a,
					existing: existing_a,
					new_kind: new_kind_a,
				},
				Self::PathCollision {
					path: path_b,
					existing: existing_b,
					new_kind: new_kind_b,
				},
			) => path_a == path_b && existing_a == existing_b && new_kind_a == new_kind_b,
			(
				Self::AssetConflict {
					path: path_a,
					expected: expected_a,
					found: found_a,
				},
				Self::AssetConflict {
					path: path_b,
					expected: expected_b,
					found: found_b,
				},
			) => path_a == path_b && expected_a == expected_b && found_a == found_b,
			(
				Self::InvalidPath {
					path: path_a,
					reason: reason_a,
				},
				Self::InvalidPath {
					path: path_b,
					reason: reason_b,
				},
			) => path_a == path_b && reason_a == reason_b,
			(
				Self::UnmeldableChartCollision {
					path: path_a,
					filename: filename_a,
				},
				Self::UnmeldableChartCollision {
					path: path_b,
					filename: filename_b,
				},
			) => path_a == path_b && filename_a == filename_b,
			_ => false,
		}
	}
}

impl Eq for IndexError {}

impl PathIndex {
	/// Create an empty index with only the root directory.
	pub fn new() -> Self {
		let mut index = Self {
			next_inode: ROOT_INODE + 1,
			nodes: BTreeMap::new(),
		};
		index.nodes.insert(
			ROOT_INODE,
			Node {
				inode: ROOT_INODE,
				kind: NodeKind::Directory,
				path: String::new(),
				parent: None,
				children: BTreeMap::new(),
			},
		);
		index
	}

	pub fn len(&self) -> usize {
		self.nodes.len()
	}

	pub fn is_empty(&self) -> bool {
		self.nodes.len() <= 1
	}

	pub fn get(&self, inode: u64) -> Option<&Node> {
		self.nodes.get(&inode)
	}

	pub fn lookup_path(&self, path: &str) -> Option<u64> {
		if path.is_empty() || path == "/" {
			return Some(ROOT_INODE);
		}
		let normalized = normalize_logical_path(path)?;
		let mut current = ROOT_INODE;
		for component in normalized.split('/') {
			let node = self.nodes.get(&current)?;
			current = *node.children.get(component)?;
		}
		Some(current)
	}

	/// Insert a chart leaf at `path`.
	pub fn insert_chart(
		&mut self,
		path: &str,
		sha256: Sha256,
		size: u64,
	) -> Result<u64, IndexError> {
		self.insert_leaf(path, NodeKind::Chart { sha256, size })
	}

	/// Insert a chart whose source is a meld of multiple bundles.
	pub fn insert_melded_chart(
		&mut self,
		path: &str,
		bundle_ids: impl Into<Box<[BundleId]>>,
	) -> Result<u64, IndexError> {
		self.insert_leaf(
			path,
			NodeKind::MeldedChart {
				bundle_ids: bundle_ids.into(),
			},
		)
	}

	/// Insert an asset leaf at `logical_path` (parent of chart + relative filename).
	pub fn insert_asset(
		&mut self,
		logical_path: &str,
		asset_id: AssetId,
		size: u64,
	) -> Result<u64, IndexError> {
		self.insert_leaf(logical_path, NodeKind::Asset { asset_id, size })
	}

	fn insert_leaf(&mut self, path: &str, kind: NodeKind) -> Result<u64, IndexError> {
		let normalized = normalize_logical_path(path).ok_or_else(|| IndexError::InvalidPath {
			path: path.to_owned(),
			reason: "path is empty or invalid".into(),
		})?;

		if normalized.is_empty() {
			return Err(IndexError::InvalidPath {
				path: path.to_owned(),
				reason: "cannot place a file at the mount root".into(),
			});
		}

		let (parent_path, name) =
			split_parent_name(&normalized).ok_or_else(|| IndexError::InvalidPath {
				path: path.to_owned(),
				reason: "missing file name".into(),
			})?;
		let name = name.to_owned();

		let parent_inode = self.ensure_directory(parent_path)?;

		if let Some(&existing_inode) = self.nodes.get(&parent_inode).unwrap().children.get(&name) {
			let existing = &self.nodes.get(&existing_inode).unwrap().kind;
			return match (existing, &kind) {
				(NodeKind::Asset { asset_id: a, .. }, NodeKind::Asset { asset_id: b, .. })
					if a == b =>
				{
					Ok(existing_inode)
				}
				(
					NodeKind::Asset {
						asset_id: expected, ..
					},
					NodeKind::Asset {
						asset_id: found, ..
					},
				) => Err(IndexError::AssetConflict {
					path: normalized,
					expected: *expected,
					found: *found,
				}),
				(existing_kind, new_kind) => Err(IndexError::PathCollision {
					path: normalized,
					existing: existing_kind.clone(),
					new_kind: new_kind.clone(),
				}),
			};
		}

		let inode = self.alloc_inode();
		self.nodes.insert(
			inode,
			Node {
				inode,
				kind,
				path: normalized,
				parent: Some(parent_inode),
				children: BTreeMap::new(),
			},
		);
		self.nodes
			.get_mut(&parent_inode)
			.expect("parent exists")
			.children
			.insert(name, inode);
		Ok(inode)
	}

	fn ensure_directory(&mut self, path: &str) -> Result<u64, IndexError> {
		if path.is_empty() {
			return Ok(ROOT_INODE);
		}

		let mut current = ROOT_INODE;
		let mut built = String::new();

		for component in path.split('/') {
			if built.is_empty() {
				built.push_str(component);
			} else {
				built.push('/');
				built.push_str(component);
			}

			if let Some(&child_inode) = self.nodes.get(&current).unwrap().children.get(component) {
				match &self.nodes.get(&child_inode).unwrap().kind {
					NodeKind::Directory => current = child_inode,
					other => {
						return Err(IndexError::PathCollision {
							path: built,
							existing: other.clone(),
							new_kind: NodeKind::Directory,
						});
					}
				}
				continue;
			}

			let inode = self.alloc_inode();
			self.nodes.insert(
				inode,
				Node {
					inode,
					kind: NodeKind::Directory,
					path: built.clone(),
					parent: Some(current),
					children: BTreeMap::new(),
				},
			);
			self.nodes
				.get_mut(&current)
				.expect("parent")
				.children
				.insert(component.to_owned(), inode);
			current = inode;
		}

		Ok(current)
	}

	fn alloc_inode(&mut self) -> u64 {
		let inode = self.next_inode;
		self.next_inode += 1;
		inode
	}

	/// Return direct child names of a directory inode, sorted.
	pub fn child_names(&self, dir_inode: u64) -> Option<Vec<(String, u64)>> {
		let node = self.nodes.get(&dir_inode)?;
		if !matches!(node.kind, NodeKind::Directory) {
			return None;
		}
		Some(
			node.children
				.iter()
				.map(|(name, inode)| (name.clone(), *inode))
				.collect(),
		)
	}
}

fn normalize_logical_path(path: &str) -> Option<String> {
	let trimmed = path.trim_matches('/');
	if trimmed.is_empty() {
		return Some(String::new());
	}

	let mut out = PathBuf::new();
	for component in Path::new(trimmed).components() {
		match component {
			Component::Normal(part) => out.push(part),
			Component::ParentDir => {
				if !out.pop() {
					return None;
				}
			}
			Component::RootDir | Component::Prefix(_) => return None,
			Component::CurDir => {}
		}
	}
	out.to_str().map(str::to_owned)
}

/// Split `path` into `(parent, name)`. A path with no `/` (a leaf placed
/// directly at the mount root, e.g. a bundle placed with an empty folder, or
/// a root-level `extra` asset) has an empty parent.
fn split_parent_name(path: &str) -> Option<(&str, &str)> {
	match path.rsplit_once('/') {
		Some((_, "")) => None,
		Some((parent, name)) => Some((parent, name)),
		None if path.is_empty() => None,
		None => Some(("", path)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use backbeat_core::Sha256;

	#[test]
	fn insert_chart_and_asset() {
		let sha = Sha256::null();
		let asset = backbeat_core::asset_id::AssetId(sha);

		let mut index = PathIndex::new();
		index
			.insert_chart("group/song.bms", sha, 456)
			.expect("insert chart");
		index
			.insert_asset("group/sound/kick.ogg", asset, 123)
			.expect("insert asset");

		let chart_inode = index.lookup_path("group/song.bms").unwrap();
		assert!(matches!(
			index.get(chart_inode).unwrap().kind,
			NodeKind::Chart { .. }
		));

		let asset_inode = index.lookup_path("group/sound/kick.ogg").unwrap();
		assert!(matches!(
			index.get(asset_inode).unwrap().kind,
			NodeKind::Asset { size: 123, .. }
		));

		let children = index
			.child_names(index.lookup_path("group").unwrap())
			.unwrap();
		let names: Vec<_> = children.iter().map(|(n, _)| n.as_str()).collect();
		assert!(names.contains(&"song.bms"));
		assert!(names.contains(&"sound"));
	}

	#[test]
	fn insert_leaf_at_mount_root() {
		let sha = Sha256::null();
		let asset = backbeat_core::asset_id::AssetId(sha);

		let mut index = PathIndex::new();
		index
			.insert_chart("root-song.bms", sha, 1)
			.expect("insert root chart");
		index
			.insert_asset("root-asset.txt", asset, 2)
			.expect("insert root asset");

		assert!(index.lookup_path("root-song.bms").is_some());
		assert!(index.lookup_path("root-asset.txt").is_some());

		let children = index.child_names(ROOT_INODE).unwrap();
		let names: Vec<_> = children.iter().map(|(n, _)| n.as_str()).collect();
		assert!(names.contains(&"root-song.bms"));
		assert!(names.contains(&"root-asset.txt"));
	}
}
