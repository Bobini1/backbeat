//! Helpers for detecting FUSE mount points.

use std::io;
use std::path::Path;

/// Returns true when `path` is already a mount point (device differs from parent).
pub fn is_mount_point(path: &Path) -> io::Result<bool> {
	use std::os::unix::fs::MetadataExt;

	let meta = std::fs::metadata(path)?;
	let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
	let parent_dev = match parent {
		Some(p) => std::fs::metadata(p)?.dev(),
		None => meta.dev(),
	};
	Ok(meta.dev() != parent_dev)
}
