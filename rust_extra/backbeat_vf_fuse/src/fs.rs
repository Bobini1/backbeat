//! [`fuser`] syscall glue over [`BackbeatFilesystem`].

use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::fs::File;
use std::io;
use std::time::Duration;

use fuser::{
	BackgroundSession, Config, Errno, FileHandle, Filesystem, FopenFlags, Generation, INodeNo,
	KernelConfig, MountOption, RenameFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
	ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr,
	Request,
};
#[cfg(target_os = "linux")]
use fuser::{CopyFileRangeFlags, InitFlags};
#[cfg(target_os = "linux")]
use parking_lot::Mutex;

#[cfg(target_os = "linux")]
use crate::passthrough::{self, BackingCache};
use backbeat_vf_fs::{Attrs, BackbeatFilesystem, FileKind};

const ATTR_TTL: Duration = Duration::from_secs(86400);

/// Keep kernel page cache across re-opens; skip flush on close (read-only export).
const OPEN_FILE_FLAGS: FopenFlags = FopenFlags::FOPEN_KEEP_CACHE.union(FopenFlags::FOPEN_NOFLUSH);

/// Let the kernel cache directory listings between readdir calls.
const OPEN_DIR_FLAGS: FopenFlags = FopenFlags::FOPEN_CACHE_DIR;

/// Pending background requests (readahead, etc.) during large scans.
const MAX_BACKGROUND: u16 = 64;

/// Queue depth at which the kernel may start backing off instead of spin-waiting.
const CONGESTION_THRESHOLD: u16 = 48;

/// FUSE adapter wrapping [`BackbeatFilesystem`].
pub struct FuseFilesystem {
	inner: BackbeatFilesystem,
	#[cfg(target_os = "linux")]
	backing_cache: Mutex<BackingCache>,
}

impl FuseFilesystem {
	pub fn new(inner: BackbeatFilesystem) -> Self {
		Self {
			inner,
			#[cfg(target_os = "linux")]
			backing_cache: Mutex::new(BackingCache::default()),
		}
	}
}

impl Filesystem for FuseFilesystem {
	fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> io::Result<()> {
		tune_kernel_config(config);
		Ok(())
	}

	fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
		let Some(name) = name.to_str() else {
			reply.error(Errno::EINVAL);
			return;
		};
		match self.inner.lookup(parent.0, name) {
			Ok(attr) => {
				let fileattr = attrs_to_fileattr(&attr);
				reply.entry(&ATTR_TTL, &fileattr, Generation(0));
			}
			Err(err) => reply.error(errno_from_io(err)),
		}
	}

	fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
		match self.inner.getattr(ino.0) {
			Ok(attr) => {
				let fileattr = attrs_to_fileattr(&attr);
				reply.attr(&ATTR_TTL, &fileattr);
			}
			Err(err) => reply.error(errno_from_io(err)),
		}
	}

	fn access(&self, _req: &Request, ino: INodeNo, _mask: fuser::AccessFlags, reply: ReplyEmpty) {
		if self.inner.index().get(ino.0).is_some() {
			reply.ok();
		} else {
			reply.error(Errno::ENOENT);
		}
	}

	fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
		let files = self.inner.index().len();
		reply.statfs(1, 0, 0, files, 0, 4096, 255, 4096);
	}

	fn flush(
		&self,
		_req: &Request,
		_ino: INodeNo,
		_fh: FileHandle,
		_lock_owner: fuser::LockOwner,
		reply: ReplyEmpty,
	) {
		reply.ok();
	}

	fn getxattr(&self, _req: &Request, ino: INodeNo, _name: &OsStr, size: u32, reply: ReplyXattr) {
		if self.inner.index().get(ino.0).is_none() {
			reply.error(Errno::ENOENT);
			return;
		}
		if size == 0 {
			reply.size(0);
		} else {
			reply.error(Errno::NO_XATTR);
		}
	}

	fn listxattr(&self, _req: &Request, ino: INodeNo, size: u32, reply: ReplyXattr) {
		if self.inner.index().get(ino.0).is_none() {
			reply.error(Errno::ENOENT);
			return;
		}
		if size == 0 {
			reply.size(0);
		} else {
			reply.data(&[]);
		}
	}

	fn setattr(
		&self,
		_req: &Request,
		_ino: INodeNo,
		_mode: Option<u32>,
		_uid: Option<u32>,
		_gid: Option<u32>,
		_size: Option<u64>,
		_atime: Option<fuser::TimeOrNow>,
		_mtime: Option<fuser::TimeOrNow>,
		_ctime: Option<std::time::SystemTime>,
		_fh: Option<FileHandle>,
		_crtime: Option<std::time::SystemTime>,
		_chgtime: Option<std::time::SystemTime>,
		_bkuptime: Option<std::time::SystemTime>,
		_flags: Option<fuser::BsdFileFlags>,
		reply: ReplyAttr,
	) {
		reply.error(Errno::EROFS);
	}

	fn open(&self, _req: &Request, ino: INodeNo, _flags: fuser::OpenFlags, reply: ReplyOpen) {
		#[cfg(target_os = "linux")]
		if passthrough::passthrough_available()
			&& let Ok(Some(path)) = self.inner.asset_backing_path(ino.0)
			&& let Ok((fh, id)) = self.backing_cache.lock().get_or_create(ino, || {
				let file = File::open(&path)?;
				reply.open_backing(&file)
			}) {
			let flags = OPEN_FILE_FLAGS.union(passthrough::passthrough_open_flags());
			reply.opened_passthrough(FileHandle(fh), flags, &id);
			return;
		}

		match self.inner.open(ino.0) {
			Ok(fh) => reply.opened(FileHandle(fh), OPEN_FILE_FLAGS),
			Err(err) => reply.error(errno_from_io(err)),
		}
	}

	fn read(
		&self,
		_req: &Request,
		_ino: INodeNo,
		fh: FileHandle,
		offset: u64,
		size: u32,
		_flags: fuser::OpenFlags,
		_lock_owner: Option<fuser::LockOwner>,
		reply: ReplyData,
	) {
		match self.inner.read(fh.0, offset, size) {
			Ok(data) => reply.data(&data),
			Err(err) => reply.error(errno_from_io(err)),
		}
	}

	fn release(
		&self,
		_req: &Request,
		_ino: INodeNo,
		fh: FileHandle,
		_flags: fuser::OpenFlags,
		_lock_owner: Option<fuser::LockOwner>,
		_flush: bool,
		reply: ReplyEmpty,
	) {
		#[cfg(target_os = "linux")]
		if self.backing_cache.lock().release(fh.0) {
			reply.ok();
			return;
		}
		self.inner.release(fh.0);
		reply.ok();
	}

	fn fsync(
		&self,
		_req: &Request,
		_ino: INodeNo,
		_fh: FileHandle,
		_datasync: bool,
		reply: ReplyEmpty,
	) {
		reply.ok();
	}

	fn opendir(&self, _req: &Request, ino: INodeNo, _flags: fuser::OpenFlags, reply: ReplyOpen) {
		match self.inner.opendir(ino.0) {
			Ok(fh) => reply.opened(FileHandle(fh), OPEN_DIR_FLAGS),
			Err(err) => reply.error(errno_from_io(err)),
		}
	}

	fn readdir(
		&self,
		_req: &Request,
		ino: INodeNo,
		_fh: FileHandle,
		offset: u64,
		mut reply: ReplyDirectory,
	) {
		let entries = match self.inner.readdir_entries(ino.0, offset) {
			Ok(entries) => entries,
			Err(err) => {
				reply.error(errno_from_io(err));
				return;
			}
		};

		for entry in entries {
			if reply.add(
				INodeNo(entry.ino),
				entry.next,
				filekind_to_filetype(entry.kind),
				&entry.name,
			) {
				break;
			}
		}
		reply.ok();
	}

	fn readdirplus(
		&self,
		_req: &Request,
		ino: INodeNo,
		_fh: FileHandle,
		offset: u64,
		mut reply: ReplyDirectoryPlus,
	) {
		let entries = match self.inner.readdir_entries(ino.0, offset) {
			Ok(entries) => entries,
			Err(err) => {
				reply.error(errno_from_io(err));
				return;
			}
		};

		for entry in entries {
			let attr = match self.inner.getattr(entry.ino) {
				Ok(attr) => attr,
				Err(err) => {
					reply.error(errno_from_io(err));
					return;
				}
			};
			let fileattr = attrs_to_fileattr(&attr);
			if reply.add(
				INodeNo(entry.ino),
				entry.next,
				&entry.name,
				&ATTR_TTL,
				&fileattr,
				Generation(0),
			) {
				break;
			}
		}
		reply.ok();
	}

	fn releasedir(
		&self,
		_req: &Request,
		_ino: INodeNo,
		fh: FileHandle,
		_flags: fuser::OpenFlags,
		reply: ReplyEmpty,
	) {
		self.inner.release(fh.0);
		reply.ok();
	}

	fn fsyncdir(
		&self,
		_req: &Request,
		_ino: INodeNo,
		_fh: FileHandle,
		_datasync: bool,
		reply: ReplyEmpty,
	) {
		reply.ok();
	}

	fn mkdir(
		&self,
		_req: &Request,
		_parent: INodeNo,
		_name: &OsStr,
		_mode: u32,
		_umask: u32,
		reply: ReplyEntry,
	) {
		reply.error(Errno::EROFS);
	}

	fn unlink(&self, _req: &Request, _parent: INodeNo, _name: &OsStr, reply: ReplyEmpty) {
		reply.error(Errno::EROFS);
	}

	fn rmdir(&self, _req: &Request, _parent: INodeNo, _name: &OsStr, reply: ReplyEmpty) {
		reply.error(Errno::EROFS);
	}

	fn rename(
		&self,
		_req: &Request,
		_parent: INodeNo,
		_name: &OsStr,
		_new_parent: INodeNo,
		_new_name: &OsStr,
		_flags: RenameFlags,
		reply: ReplyEmpty,
	) {
		reply.error(Errno::EROFS);
	}

	fn create(
		&self,
		_req: &Request,
		_parent: INodeNo,
		_name: &OsStr,
		_mode: u32,
		_umask: u32,
		_flags: i32,
		reply: ReplyCreate,
	) {
		reply.error(Errno::EROFS);
	}

	fn write(
		&self,
		_req: &Request,
		_ino: INodeNo,
		_fh: FileHandle,
		_offset: u64,
		_data: &[u8],
		_write_flags: fuser::WriteFlags,
		_flags: fuser::OpenFlags,
		_lock_owner: Option<fuser::LockOwner>,
		reply: ReplyWrite,
	) {
		reply.error(Errno::EROFS);
	}

	#[cfg(target_os = "linux")]
	fn copy_file_range(
		&self,
		_req: &Request,
		_ino_in: INodeNo,
		fh_in: FileHandle,
		offset_in: u64,
		_ino_out: INodeNo,
		fh_out: FileHandle,
		offset_out: u64,
		len: u64,
		_flags: CopyFileRangeFlags,
		reply: ReplyWrite,
	) {
		// Both inodes live on this read-only export; copying onto the mount is a write.
		if self.inner.index().get(_ino_out.0).is_some() {
			reply.error(Errno::EROFS);
			return;
		}

		// Passthrough handles are kernel-served; only FUSE-managed file handles reach here.
		{
			let cache = self.backing_cache.lock();
			if cache.is_passthrough(fh_in.0) || cache.is_passthrough(fh_out.0) {
				reply.error(Errno::ENOSYS);
				return;
			}
		}

		match self
			.inner
			.copy_file_range_between_handles(fh_in.0, offset_in, fh_out.0, offset_out, len)
		{
			Ok(n) => reply.written(n),
			Err(err) => reply.error(errno_from_io(err)),
		}
	}
}

fn tune_kernel_config(config: &mut KernelConfig) {
	// Use the largest readahead the kernel will accept.
	let _ = config.set_max_readahead(u32::MAX);
	let _ = config.set_max_background(MAX_BACKGROUND);
	let _ = config.set_congestion_threshold(CONGESTION_THRESHOLD);

	#[cfg(target_os = "linux")]
	{
		let _ = config.add_capabilities(InitFlags::FUSE_PASSTHROUGH);
		let _ = config.set_max_stack_depth(2);
	}
}

fn attrs_to_fileattr(a: &Attrs) -> fuser::FileAttr {
	fuser::FileAttr {
		ino: fuser::INodeNo(a.ino),
		size: a.size,
		blocks: a.blocks,
		atime: a.atime,
		mtime: a.mtime,
		ctime: a.ctime,
		crtime: a.crtime,
		kind: filekind_to_filetype(a.kind),
		perm: a.perm,
		nlink: a.nlink,
		uid: owner_uid(),
		gid: owner_gid(),
		rdev: a.rdev,
		flags: 0,
		blksize: a.blksize,
	}
}

fn filekind_to_filetype(k: FileKind) -> fuser::FileType {
	match k {
		FileKind::Directory => fuser::FileType::Directory,
		FileKind::RegularFile => fuser::FileType::RegularFile,
	}
}

fn owner_uid() -> u32 {
	nix::unistd::getuid().as_raw()
}

fn owner_gid() -> u32 {
	nix::unistd::getgid().as_raw()
}

fn errno_from_io(err: io::Error) -> Errno {
	if let Some(code) = err.raw_os_error() {
		return Errno::from_i32(code);
	}
	match err.kind() {
		io::ErrorKind::NotFound => Errno::ENOENT,
		io::ErrorKind::PermissionDenied => Errno::EACCES,
		io::ErrorKind::AlreadyExists => Errno::EEXIST,
		io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => Errno::EINVAL,
		io::ErrorKind::Interrupted => Errno::EINTR,
		_ => Errno::EIO,
	}
}

/// Default mount options for the Backbeat export filesystem.
pub fn default_mount_options() -> Vec<MountOption> {
	vec![
		MountOption::FSName("backbeat".into()),
		MountOption::RO,
		MountOption::DefaultPermissions,
	]
}

/// Default fuser session configuration for the Backbeat export filesystem.
pub fn default_fuse_config() -> Config {
	let mut config = Config::default();
	config.mount_options = default_mount_options();
	config
}

/// Returns true when a FUSE backend appears to be available on this system.
pub fn fuse_available() -> bool {
	if std::env::var_os("BACKBEAT_VF_EMULATE_FUSE_UNAVAILABLE").is_some() {
		return false;
	}

	#[cfg(target_os = "linux")]
	{
		std::path::Path::new("/dev/fuse").exists()
	}
	#[cfg(target_os = "macos")]
	{
		std::path::Path::new("/Library/Filesystems/macfuse.fs").exists()
			|| std::path::Path::new("/Library/Filesystems/osxfuse.fs").exists()
	}
	#[cfg(not(any(target_os = "linux", target_os = "macos")))]
	{
		false
	}
}

/// Mount the filesystem and return a background session handle.
pub fn spawn_mount(
	filesystem: FuseFilesystem,
	mountpoint: impl AsRef<std::path::Path>,
	config: &Config,
) -> io::Result<BackgroundSession> {
	fuser::spawn_mount2(filesystem, mountpoint, config)
}
