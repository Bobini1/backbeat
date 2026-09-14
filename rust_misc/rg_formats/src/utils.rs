use std::ffi::OsString;
use std::path::Path;

/// Return the extension of `path` as a `&str`, correctly handling dotfiles.
///
/// [`Path::extension`] returns `None` for a file whose name begins with `.`
/// and contains no other dots. This is ridiculous behaviour.
pub(crate) fn safe_extension(path: &Path) -> Option<&str> {
	if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
		return Some(ext);
	}
	let name = path.file_name()?.to_str()?;
	let rest = name.strip_prefix('.')?;
	if !rest.is_empty() && !rest.contains('.') {
		Some(rest)
	} else {
		None
	}
}

pub(crate) fn os_string_from_bytes(bytes: &[u8]) -> OsString {
	#[cfg(unix)]
	{
		use std::os::unix::ffi::OsStrExt;

		std::ffi::OsStr::from_bytes(bytes).to_owned()
	}

	#[cfg(windows)]
	{
		use std::os::windows::ffi::OsStringExt;

		match String::from_utf8(bytes.to_vec()) {
			Ok(value) => OsString::from(value),
			Err(_) => {
				OsString::from_wide(&bytes.iter().map(|&byte| byte as u16).collect::<Vec<_>>())
			}
		}
	}
}

/// A struct for easier iteration over a series of bytes.
///
/// This provides general methods -- peek, prev, etc.
pub(crate) struct ByteIterator<'a> {
	buf: &'a [u8],
	index: usize,
}

impl ByteIterator<'_> {
	pub(crate) fn new(buf: &[u8]) -> ByteIterator<'_> {
		ByteIterator { buf, index: 0 }
	}

	/// Rewind the index by one.
	pub(crate) fn rewind(&mut self) {
		self.index -= 1;
	}

	#[allow(unused)]
	pub(crate) fn set_index(&mut self, new_index: usize) {
		self.index = new_index;
	}

	#[allow(unused)]
	pub(crate) fn is_eof(&self) -> bool {
		self.index >= self.buf.len()
	}

	/// Peek at the previous byte.
	pub(crate) fn prev(&self) -> Option<&u8> {
		self.buf.get(self.index - 1)
	}

	/// Read the byte at the current index. Increments the index.
	/// Returns none if at EOF.
	pub(crate) fn read(&mut self) -> Option<&u8> {
		let byte = self.buf.get(self.index);

		self.index += 1;

		byte
	}

	/// Looks at the current character under the cursor without incrementing the index.
	pub(crate) fn peek(&self) -> Option<&u8> {
		self.buf.get(self.index)
	}
}

/// Convenience macro for reading a value and breaking out of the loop
/// if we're at eof.
#[doc(hidden)]
macro_rules! read {
	($b_iter:expr) => {{
		let v = *match $b_iter.read() {
			Some(byte) => byte,
			None => break,
		};

		v
	}};
}

pub(crate) use read;

/// We're in a scenario where we can't guarantee that this input is valid utf8, and
/// therefore cannot use a [`String`] here.
pub(crate) type ByteString = Box<[u8]>;
