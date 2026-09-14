#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
// The entire crate body is Windows-only. On every other target this file
// expands to nothing, so `cargo check -p backbeat_vf_winfsp` produces an
// empty library and never pulls in the (Windows-only) `winfsp` binding.
#![cfg(target_os = "windows")]

//! Windows (`WinFsp`) mount adapter over [`backbeat_vf_fs`].
//!
//! Mounts a [`backbeat_vf_fs::BackbeatFilesystem`] as a read-only Windows
//! filesystem using the [`winfsp`] crate. All mutating operations return
//! `STATUS_ACCESS_DENIED`; the volume is also advertised read-only to the
//! kernel via [`winfsp::host::VolumeParams::read_only_volume`], so the driver
//! rejects most write intents before they reach us.

use std::collections::HashMap;
use std::ffi::c_void;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{SystemTime, UNIX_EPOCH};

use winfsp::filesystem::{
	DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, ModificationDescriptor,
	OpenFileInfo, VolumeInfo, WideNameInfo,
};
use winfsp::host::{FileSystemHost, FileSystemParams, VolumeParams};
use winfsp::{FspError, FspInit, Result as WinResult, U16CStr, winfsp_init, winfsp_init_or_die};

use backbeat_vf_fs::{Attrs, BackbeatFilesystem, DentryIndex, FileKind, MountError, ROOT_INODE};

// ---------------------------------------------------------------------------
// NTSTATUS codes
//
// `winfsp::constants` does not re-export NTSTATUS names, and `winfsp-sys` is
// not a direct dependency of this crate. We therefore express the few
// NTSTATUS values we need as raw `i32` and wrap them in `FspError::NTSTATUS`.
// The literals are the canonical Windows values; see
// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-erref/596a1078-e883-4972-9b83-00398d286f18
//
// TODO(winfsp): verify these NTSTATUS integer values against the build.
const STATUS_INVALID_HANDLE: i32 = 0xC0000008_u32 as i32;
const STATUS_INVALID_DEVICE_REQUEST: i32 = 0xC0000010_u32 as i32;
const STATUS_ACCESS_DENIED: i32 = 0xC0000021_u32 as i32;
const STATUS_BUFFER_TOO_SMALL: i32 = 0xC0000023_u32 as i32;
const STATUS_END_OF_FILE: i32 = 0xC0000011_u32 as i32;
const STATUS_FILE_IS_A_DIRECTORY: i32 = 0xC00000BA_u32 as i32;
const STATUS_NOT_A_DIRECTORY: i32 = 0xC0000103_u32 as i32;
const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC0000034_u32 as i32;
const STATUS_OBJECT_PATH_NOT_FOUND: i32 = 0xC000003A_u32 as i32;

// Windows file attribute flags (subset).
const FILE_ATTRIBUTE_READONLY: u32 = 0x0001;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;

/// 100-nanosecond intervals between 1601-01-01 and 1970-01-01 (the FILETIME
/// epoch offset from the Unix epoch).
const FILETIME_EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;

// ---------------------------------------------------------------------------
// Public surface (mirrors `backbeat_vf_fuse` so the CLI can alias uniformly)

/// Configuration for mounting a filesystem. WinFsp v1 does not expose the
/// cross-user ACL knob that FUSE's `allow_other` maps to, so `allow_other`
/// is accepted for API parity but currently a no-op.
#[derive(Debug, Clone, Default)]
pub struct MountConfig {
	/// Accepted for parity with the FUSE adapter; ignored on Windows.
	pub allow_other: bool,
}

/// Handle keeping a WinFsp mount alive. Dropping it unmounts and stops the
/// filesystem host (its [`FileSystemHost`] `Drop` performs `unmount` + `stop`).
///
/// Field declaration order matters: `_host` is dropped *before* `_init`, so the
/// host is torn down while the WinFsp runtime token is still live.
pub struct MountHandle {
	_host: FileSystemHost<WinFspFilesystem>,
	_init: FspInit,
}

/// Returns `true` when the WinFsp runtime can be initialized on this machine.
pub fn fuse_available() -> bool {
	if std::env::var_os("BACKBEAT_VF_EMULATE_FUSE_UNAVAILABLE").is_some() {
		return false;
	}

	// `winfsp_init` returns a token whose drop releases WinFsp; a successful
	// call (and immediate release) is enough to detect an installed runtime.
	winfsp_init().is_ok()
}

/// Mount `filesystem` at `mountpoint`.
///
/// Returns a [`MountHandle`] that keeps the mount alive until dropped. WinFsp
/// dispatches operations on kernel-managed threads, so no worker thread is
/// spawned here.
pub fn mount(
	filesystem: BackbeatFilesystem,
	mountpoint: impl AsRef<Path>,
	_config: MountConfig,
) -> Result<MountHandle, MountError> {
	std::fs::create_dir_all(mountpoint.as_ref())?;

	let init = winfsp_init_or_die();
	let ctx = WinFspFilesystem::new(filesystem);

	let volume_params = build_volume_params();
	// TODO(winfsp): verify `FileSystemParams` field is settable this way and
	// that `use_dir_info_by_name` is what enables `get_dir_info_by_name`.
	let mut options = FileSystemParams::default_params(volume_params);
	options.use_dir_info_by_name = true;

	let mut host = FileSystemHost::new_with_options(options, ctx)
		.map_err(|e| MountError::Mount(io::Error::other(e)))?;
	host.mount(mountpoint.as_ref())
		.map_err(|e| MountError::Mount(io::Error::other(e)))?;
	host.start()
		.map_err(|e| MountError::Mount(io::Error::other(e)))?;

	Ok(MountHandle {
		_host: host,
		_init: init,
	})
}

// ---------------------------------------------------------------------------
// WinFsp filesystem context

/// WinFsp adapter wrapping a [`BackbeatFilesystem`].
///
/// `FileContext` is the platform-neutral file handle (`fh`, a `u64`). We keep a
/// map from `fh` back to its inode so the per-handle callbacks
/// (`get_file_info`, `read_directory`, ...) can recover the inode without
/// re-walking the path.
pub struct WinFspFilesystem {
	inner: Arc<BackbeatFilesystem>,
	fh_to_inode: StdMutex<HashMap<u64, u64>>,
}

impl WinFspFilesystem {
	fn new(inner: BackbeatFilesystem) -> Self {
		Self {
			inner: Arc::new(inner),
			fh_to_inode: StdMutex::new(HashMap::new()),
		}
	}

	/// Resolve a WinFsp path (backslash-separated, relative to the volume root,
	/// e.g. `\foo\bar`) to an inode by walking `lookup_child` from
	/// [`ROOT_INODE`]. Returns `None` if any component is missing.
	fn resolve_path(&self, file_name: &U16CStr) -> Option<u64> {
		let s = file_name.to_string_lossy();
		let mut current = ROOT_INODE;
		for component in s.split('\\') {
			if component.is_empty() {
				// Leading/back-to-back backslashes and the volume root itself.
				continue;
			}
			current = self.inner.index().lookup_child(current, component)?;
		}
		Some(current)
	}
}

impl FileSystemContext for WinFspFilesystem {
	type FileContext = u64;

	fn get_security_by_name(
		&self,
		file_name: &U16CStr,
		_security_descriptor: Option<&mut [c_void]>,
		_reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
	) -> WinResult<FileSecurity> {
		// TODO(winfsp): a real security descriptor is not produced here. We
		// report attributes only and a zero-length descriptor, letting the
		// driver fall back to its default (volume-owned) security.
		let inode = self
			.resolve_path(file_name)
			.ok_or_else(|| FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND))?;
		let attrs = self.inner.getattr(inode).map_err(fsp_from_io)?;
		Ok(FileSecurity {
			reparse: false,
			sz_security_descriptor: 0,
			attributes: attrs_to_file_attributes(&attrs),
		})
	}

	fn open(
		&self,
		file_name: &U16CStr,
		_create_options: u32,
		// `FILE_ACCESS_RIGHTS` is a transparent `u32` alias in winfsp-sys, so a
		// literal `u32` signature satisfies the trait without naming
		// `winfsp_sys` (not a direct dependency of this crate).
		_granted_access: u32,
		file_info: &mut OpenFileInfo,
	) -> WinResult<u64> {
		// TODO(winfsp): reject write/modification intents explicitly once the
		// `FILE_ACCESS_RIGHTS` bit constants are nameable. The driver rejects
		// writes regardless because `read_only_volume(true)` is set.
		let inode = self
			.resolve_path(file_name)
			.ok_or_else(|| FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND))?;
		let attrs = self.inner.getattr(inode).map_err(fsp_from_io)?;

		let fh = match attrs.kind {
			FileKind::Directory => self.inner.opendir(inode).map_err(fsp_from_io)?,
			FileKind::RegularFile => self.inner.open(inode).map_err(fsp_from_io)?,
		};
		self.fh_to_inode.lock().unwrap().insert(fh, inode);

		// `OpenFileInfo: AsMut<FileInfo>` exposes the file info to fill.
		attrs_to_file_info(&attrs, file_info.as_mut());
		Ok(fh)
	}

	fn close(&self, context: u64) {
		self.inner.release(context);
		self.fh_to_inode.lock().unwrap().remove(&context);
	}

	fn cleanup(&self, _context: &u64, _file_name: Option<&U16CStr>, _flags: u32) {
		// Nothing to do on a read-only volume; handles are released in `close`.
	}

	fn read(&self, context: &u64, buffer: &mut [u8], offset: u64) -> WinResult<u32> {
		let data = self
			.inner
			.read(*context, offset, buffer.len() as u32)
			.map_err(fsp_from_io)?;
		let n = data.len().min(buffer.len());
		buffer[..n].copy_from_slice(&data[..n]);
		Ok(n as u32)
	}

	fn read_directory(
		&self,
		context: &u64,
		_pattern: Option<&U16CStr>,
		marker: DirMarker,
		buffer: &mut [u8],
	) -> WinResult<u32> {
		let inode = self
			.fh_to_inode
			.lock()
			.unwrap()
			.get(context)
			.copied()
			.ok_or_else(|| FspError::NTSTATUS(STATUS_INVALID_HANDLE))?;

		// Full rescan each call: simplest and correct. `readdir_entries`
		// already includes `.` and `..` as the first two entries.
		let entries = self.inner.readdir_entries(inode, 0).map_err(fsp_from_io)?;

		// Determine the resume index implied by the marker.
		let start = if marker.is_none() {
			0
		} else if marker.is_current() {
			// Resume after `.` -> start at `..`.
			1
		} else if marker.is_parent() {
			// Resume after `..` -> start at first real child.
			2
		} else if let Some(name) = marker.inner_as_cstr() {
			let needle = name.to_string_lossy();
			let mut idx = entries.len();
			for (i, e) in entries.iter().enumerate() {
				if e.name == needle {
					idx = i + 1;
					break;
				}
			}
			idx
		} else {
			0
		};

		let mut cursor: u32 = 0;
		for entry in entries.into_iter().skip(start) {
			let attrs = match self.inner.getattr(entry.ino) {
				Ok(a) => a,
				Err(err) => return Err(fsp_from_io(err)),
			};

			let mut dir_info = DirInfo::new();
			attrs_to_file_info(&attrs, dir_info.file_info_mut());
			// TODO(winfsp): verify the `&U16CStr`/`AsRef<OsStr>` (or `cstr`)
			// promotion for `set_name` matches the trait's bound exactly.
			dir_info.set_name(&entry.name).map_err(fsp_from_io)?;

			if !dir_info.append_to_buffer(buffer, &mut cursor) {
				// Buffer full: stop here, the driver calls back for the rest.
				break;
			}
		}

		if !DirInfo::finalize_buffer(buffer, &mut cursor) {
			// Cannot fit even the terminating zero entry; ask the driver for a
			// larger buffer by signalling buffer-too-small.
			return Err(FspError::NTSTATUS(STATUS_BUFFER_TOO_SMALL));
		}

		Ok(cursor)
	}

	fn get_dir_info_by_name(
		&self,
		context: &u64,
		file_name: &U16CStr,
		out_dir_info: &mut DirInfo,
	) -> WinResult<()> {
		let parent = self
			.fh_to_inode
			.lock()
			.unwrap()
			.get(context)
			.copied()
			.ok_or_else(|| FspError::NTSTATUS(STATUS_INVALID_HANDLE))?;

		let name = file_name.to_string_lossy();
		let child = self
			.inner
			.index()
			.lookup_child(parent, &name)
			.ok_or_else(|| FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND))?;
		let attrs = self.inner.getattr(child).map_err(fsp_from_io)?;
		attrs_to_file_info(&attrs, out_dir_info.file_info_mut());
		// `set_name_cstr` keeps the exact wide name (`AsRef<U16CStr>`).
		out_dir_info.set_name_cstr(file_name).map_err(fsp_from_io)?;
		Ok(())
	}

	fn get_file_info(&self, context: &u64, file_info: &mut FileInfo) -> WinResult<()> {
		let inode = self
			.fh_to_inode
			.lock()
			.unwrap()
			.get(context)
			.copied()
			.ok_or_else(|| FspError::NTSTATUS(STATUS_INVALID_HANDLE))?;
		let attrs = self.inner.getattr(inode).map_err(fsp_from_io)?;
		attrs_to_file_info(&attrs, file_info);
		Ok(())
	}

	fn get_volume_info(&self, out_volume_info: &mut VolumeInfo) -> WinResult<()> {
		// Report a large, fully-occupied read-only volume.
		out_volume_info.total_size = 16 * 1024 * 1024 * 1024; // 16 GiB
		out_volume_info.free_size = 0;
		out_volume_info.set_volume_label("backbeat");
		Ok(())
	}

	// ----- read-only: every mutating operation is rejected. -----

	// `FILE_FLAGS_AND_ATTRIBUTES` / `FILE_ACCESS_RIGHTS` are transparent `u32`
	// aliases in winfsp-sys, so `u32` satisfies these signatures.
	#[allow(clippy::too_many_arguments)]
	fn create(
		&self,
		_file_name: &U16CStr,
		_create_options: u32,
		_granted_access: u32,
		_file_attributes: u32,
		_security_descriptor: Option<&[c_void]>,
		_allocation_size: u64,
		_extra_buffer: Option<&[u8]>,
		_extra_buffer_is_reparse_point: bool,
		_file_info: &mut OpenFileInfo,
	) -> WinResult<u64> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn overwrite(
		&self,
		_context: &u64,
		_file_attributes: u32,
		_replace_file_attributes: bool,
		_allocation_size: u64,
		_extra_buffer: Option<&[u8]>,
		_file_info: &mut FileInfo,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn write(
		&self,
		_context: &u64,
		_buffer: &[u8],
		_offset: u64,
		_write_to_eof: bool,
		_constrained_io: bool,
		_file_info: &mut FileInfo,
	) -> WinResult<u32> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn flush(&self, _context: Option<&u64>, _file_info: &mut FileInfo) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn rename(
		&self,
		_context: &u64,
		_file_name: &U16CStr,
		_new_file_name: &U16CStr,
		_replace_if_exists: bool,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_security(
		&self,
		_context: &u64,
		_security_information: u32,
		_modification_descriptor: ModificationDescriptor,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	#[allow(clippy::too_many_arguments)]
	fn set_basic_info(
		&self,
		_context: &u64,
		_file_attributes: u32,
		_creation_time: u64,
		_last_access_time: u64,
		_last_write_time: u64,
		_last_change_time: u64,
		_file_info: &mut FileInfo,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_delete(
		&self,
		_context: &u64,
		_file_name: &U16CStr,
		_delete_file: bool,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_file_size(
		&self,
		_context: &u64,
		_new_size: u64,
		_set_allocation_size: bool,
		_file_info: &mut FileInfo,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_volume_label(
		&self,
		_volume_label: &U16CStr,
		_volume_info: &mut VolumeInfo,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn get_stream_info(&self, _context: &u64, _buffer: &mut [u8]) -> WinResult<u32> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn get_reparse_point(
		&self,
		_context: &u64,
		_file_name: &U16CStr,
		_buffer: &mut [u8],
	) -> WinResult<u64> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn get_reparse_point_by_name(
		&self,
		_file_name: &U16CStr,
		_is_directory: bool,
		_buffer: &mut [u8],
	) -> WinResult<u64> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_reparse_point(
		&self,
		_context: &u64,
		_file_name: &U16CStr,
		_buffer: &[u8],
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn delete_reparse_point(
		&self,
		_context: &u64,
		_file_name: &U16CStr,
		_buffer: &[u8],
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn get_extended_attributes(&self, _context: &u64, _buffer: &mut [u8]) -> WinResult<u32> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn set_extended_attributes(
		&self,
		_context: &u64,
		_buffer: &[u8],
		_file_info: &mut FileInfo,
	) -> WinResult<()> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}

	fn control(
		&self,
		_context: &u64,
		_control_code: u32,
		_input: &[u8],
		_output: &mut [u8],
	) -> WinResult<u32> {
		Err(FspError::NTSTATUS(STATUS_ACCESS_DENIED))
	}
}

// ---------------------------------------------------------------------------
// Conversions

fn build_volume_params() -> VolumeParams {
	let mut params = VolumeParams::new();
	params
		.read_only_volume(true)
		.case_sensitive_search(false)
		.case_preserved_names(true)
		.unicode_on_disk(true)
		.max_component_length(255)
		// TODO(winfsp): verify these builder setters against the winfsp 0.13
		// `VolumeParams` docs (taken from docs.rs).
		.sector_size(4096)
		.sectors_per_allocation_unit(1)
		.volume_serial_number(0xBAC0_BEA0)
		.volume_creation_time(systemtime_to_filetime(SystemTime::now()))
		.file_info_timeout(u32::MAX)
		.volume_info_timeout(u32::MAX)
		.pass_query_directory_filename(true)
		.filesystem_name("backbeat");
	params
}

fn attrs_to_file_attributes(a: &Attrs) -> u32 {
	match a.kind {
		FileKind::Directory => FILE_ATTRIBUTE_DIRECTORY,
		FileKind::RegularFile => FILE_ATTRIBUTE_READONLY,
	}
}

/// Translate [`Attrs`] into a WinFsp [`FileInfo`].
fn attrs_to_file_info(a: &Attrs, fi: &mut FileInfo) {
	fi.file_attributes = attrs_to_file_attributes(a);
	fi.reparse_tag = 0;
	fi.file_size = if matches!(a.kind, FileKind::Directory) {
		0
	} else {
		a.size
	};
	fi.allocation_size = if matches!(a.kind, FileKind::Directory) {
		0
	} else {
		// Round up to a 4 KiB allocation unit (matches the reported sector size).
		(a.size + 4095) / 4096 * 4096
	};
	fi.creation_time = systemtime_to_filetime(a.crtime);
	fi.last_access_time = systemtime_to_filetime(a.atime);
	fi.last_write_time = systemtime_to_filetime(a.mtime);
	fi.change_time = systemtime_to_filetime(a.ctime);
	fi.index_number = a.ino;
	fi.hard_links = 0;
	fi.ea_size = 0;
}

/// Convert a [`SystemTime`] to a Windows FILETIME (100-ns intervals since
/// 1601-01-01).
// TODO(winfsp): verify FILETIME conversion against the WinFsp examples.
fn systemtime_to_filetime(t: SystemTime) -> u64 {
	match t.duration_since(UNIX_EPOCH) {
		Ok(d) => {
			// d.as_nanos() is u128; /100 gives 100-ns intervals.
			FILETIME_EPOCH_DIFF_100NS + (d.as_nanos() as u64 / 100)
		}
		Err(e) => {
			// `t` predates the Unix epoch: subtract the deficit.
			let deficit = e.duration().as_nanos() as u64 / 100;
			FILETIME_EPOCH_DIFF_100NS.saturating_sub(deficit)
		}
	}
}

/// Map a platform-neutral [`io::Error`] into a WinFsp [`FspError`], preferring
/// explicit NTSTATUS values for the common NotFound cases.
fn fsp_from_io(err: io::Error) -> FspError {
	match err.kind() {
		io::ErrorKind::NotFound => FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND),
		io::ErrorKind::PermissionDenied => FspError::NTSTATUS(STATUS_ACCESS_DENIED),
		// `IsADirectory`/`NotADirectory` are not stabilized across rustc
		// versions; let the winfsp crate map the rest via `From<io::Error>`.
		_ => FspError::from(err),
	}
}
