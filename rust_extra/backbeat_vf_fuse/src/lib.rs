#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![forbid(unsafe_code)]

//! Unix (`fuser`) mount adapter over [`backbeat_vf_fs`].
//!
//! Mounts a [`backbeat_vf_fs::BackbeatFilesystem`] as a read-only FUSE filesystem
//! on Linux and macOS. Windows is served by the `backbeat_vf_winfsp` crate.

mod fs;
mod mount_util;
#[cfg(target_os = "linux")]
mod passthrough;

pub use self::fs::{
	FuseFilesystem, default_fuse_config, default_mount_options, fuse_available, spawn_mount,
};
pub use self::mount_util::is_mount_point;
pub use backbeat_vf_fs::{
	BackbeatFilesystem, DentryIndex, DentryNode, IndexError, MemDentryIndex, Node, NodeKind,
	PathIndex, ROOT_INODE, open_index, prepare,
};
pub use fuser::{BackgroundSession, Config, MountOption, SessionACL};

use std::path::Path;
/// Handle keeping a FUSE mount alive. Drop or unmount to tear down.
pub type MountHandle = BackgroundSession;

/// Errors from preparing or mounting the filesystem.
#[derive(Debug, thiserror::Error)]
pub enum FuseMountError {
	#[error("FUSE is not available on this system")]
	FuseUnavailable,

	#[error("{path} is already a mount point; unmount it first with `bkb unmount`")]
	AlreadyMounted { path: String },

	#[error("mount failed: {0}")]
	Mount(#[from] std::io::Error),
}

/// Configuration for mounting a filesystem.
#[derive(Debug, Clone)]
pub struct MountConfig {
	pub fuse_config: Config,
}

impl Default for MountConfig {
	fn default() -> Self {
		Self {
			fuse_config: default_fuse_config(),
		}
	}
}

/// Mount `filesystem` at `mountpoint`.
///
/// Returns a [`MountHandle`] that keeps the session alive until dropped.
/// The mount runs on a background thread inside [`fuser`].
pub fn mount(
	filesystem: BackbeatFilesystem,
	mountpoint: impl AsRef<Path>,
	config: MountConfig,
) -> Result<MountHandle, FuseMountError> {
	if !fuse_available() {
		return Err(FuseMountError::FuseUnavailable);
	}

	let adapter = FuseFilesystem::new(filesystem);

	let mountpoint = mountpoint.as_ref();
	std::fs::create_dir_all(mountpoint)?;
	if is_mount_point(mountpoint)? {
		return Err(FuseMountError::AlreadyMounted {
			path: mountpoint.display().to_string(),
		});
	}

	spawn_mount(adapter, mountpoint, &config.fuse_config).map_err(FuseMountError::Mount)
}
