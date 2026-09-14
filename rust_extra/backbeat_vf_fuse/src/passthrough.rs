//! Linux FUSE fd passthrough: kernel reads backing `assets/` files directly.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use fuser::BackingId;
use fuser::INodeNo;

const PASSTHROUGH_HANDLE_TAG: u64 = 1 << 63;

/// Tracks [`BackingId`] handles for passthrough opens.
#[derive(Debug, Default)]
pub struct BackingCache {
	by_handle: HashMap<u64, Arc<BackingId>>,
	by_inode: HashMap<INodeNo, Weak<BackingId>>,
	next_fh: u64,
}

impl BackingCache {
	pub fn get_or_create(
		&mut self,
		ino: INodeNo,
		create: impl FnOnce() -> std::io::Result<BackingId>,
	) -> std::io::Result<(u64, Arc<BackingId>)> {
		let fh = passthrough_handle(self.next_fh)?;
		self.next_fh += 1;

		let id = if let Some(id) = self.by_inode.get(&ino).and_then(Weak::upgrade) {
			id
		} else {
			let id = Arc::new(create()?);
			self.by_inode.insert(ino, Arc::downgrade(&id));
			id
		};

		self.by_handle.insert(fh, Arc::clone(&id));
		Ok((fh, id))
	}

	pub fn release(&mut self, fh: u64) -> bool {
		self.by_handle.remove(&fh).is_some()
	}

	pub fn is_passthrough(&self, fh: u64) -> bool {
		self.by_handle.contains_key(&fh)
	}
}

fn passthrough_handle(index: u64) -> std::io::Result<u64> {
	if index >= PASSTHROUGH_HANDLE_TAG {
		return Err(std::io::Error::other("passthrough handle space exhausted"));
	}
	Ok(PASSTHROUGH_HANDLE_TAG | index)
}

/// Whether fd passthrough is supported on this platform.
pub const fn passthrough_available() -> bool {
	cfg!(target_os = "linux")
}

/// Open flags OR'd into passthrough replies.
pub fn passthrough_open_flags() -> fuser::FopenFlags {
	fuser::FopenFlags::FOPEN_PASSTHROUGH
}

#[cfg(test)]
mod tests {
	use super::{PASSTHROUGH_HANDLE_TAG, passthrough_handle};

	#[test]
	fn passthrough_handles_use_the_tagged_namespace() {
		assert_eq!(passthrough_handle(0).unwrap(), PASSTHROUGH_HANDLE_TAG);
		assert_eq!(passthrough_handle(1).unwrap(), PASSTHROUGH_HANDLE_TAG | 1);
		assert!(passthrough_handle(PASSTHROUGH_HANDLE_TAG).is_err());
	}
}
