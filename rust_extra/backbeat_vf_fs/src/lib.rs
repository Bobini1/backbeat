#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
//! Platform-neutral filesystem logic shared by the [`backbeat_vf_fuse`] (Unix,
//! `fuser`) and [`backbeat_vf_winfsp`] (Windows, WinFsp) mount adapters.
//!
//! [`backbeat_vf_fuse`]: ../../backbeat_vf_fuse
//! [`backbeat_vf_winfsp`]: ../../backbeat_vf_winfsp
//!
//! # Platform support
//!
//! This crate contains no platform-specific glue and builds on every target.
//! Adapters translate [`Attrs`] / [`DirEntry`] into their host filesystem API
//! ([`fuser::FileAttr`] on Unix, WinFsp `FileInfo` on Windows).

#![forbid(unsafe_code)]

pub mod bms;
mod database;
mod dentry_backend;
mod index;
mod jails;
pub mod kshoot;
mod model;
mod safe_paths;
pub mod stepmania;
// mod scan;

pub use self::dentry_backend::{DentryIndex, DentryNode, MemDentryIndex, open_index};
pub use self::index::{IndexError, Node, NodeKind, PathIndex, ROOT_INODE};
pub use self::model::BackbeatFilesystem;
// pub use scan::{build_index, build_index_from_snapshot};

use std::io;
use std::sync::Arc;
use std::time::SystemTime;

use backbeat_sdk::StoreError;

/// Backend-neutral file kind: directory or regular file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
	Directory,
	RegularFile,
}

/// Backend-neutral inode metadata returned by [`BackbeatFilesystem::getattr`].
///
/// `uid` / `gid` are `None` when the backend has no Unix owner concept; an
/// adapter fills in host-native semantics (Windows uses a security descriptor).
#[derive(Debug, Clone)]
pub struct Attrs {
	pub ino: u64,
	pub size: u64,
	pub kind: FileKind,
	pub nlink: u32,
	pub perm: u16,
	pub atime: SystemTime,
	pub mtime: SystemTime,
	pub ctime: SystemTime,
	pub crtime: SystemTime,
	pub uid: Option<u32>,
	pub gid: Option<u32>,
	pub blocks: u64,
	pub blksize: u32,
	pub rdev: u32,
}

/// One directory entry returned by [`BackbeatFilesystem::readdir_entries`].
#[derive(Debug, Clone)]
pub struct DirEntry {
	pub ino: u64,
	pub kind: FileKind,
	pub name: String,
	/// 1-based next offset to pass as the next readdir `offset`.
	pub next: u64,
}

/// Errors from preparing or mounting the filesystem.
#[derive(Debug, thiserror::Error)]
pub enum MountError {
	#[error("failed to build export index: {0}")]
	Index(#[from] IndexError),

	#[error("store error: {0}")]
	Store(#[from] StoreError),

	#[error("filesystem backend is not available on this system")]
	BackendUnavailable,

	#[error("{path} is already a mount point")]
	AlreadyMounted { path: String },

	#[error("mount failed: {0}")]
	Mount(#[from] io::Error),
}

/// Platform-neutral mount configuration. Adapters wrap this with their own
/// backend-specific options.
#[derive(Debug, Clone, Default)]
pub struct MountConfig {
	/// Allow other users to access the mount. No-op on backends without an
	/// equivalent knob (e.g. WinFsp cross-user ACLs in v1).
	pub allow_other: bool,
}

/// Prepare a [`BackbeatFilesystem`] without mounting (used in tests).
pub fn prepare(
	store: backbeat_sdk::Backbeat,
	source: &str,
) -> Result<BackbeatFilesystem, MountError> {
	let index = Arc::new(open_index(&store, source)?);
	Ok(BackbeatFilesystem::new(index, store))
}
