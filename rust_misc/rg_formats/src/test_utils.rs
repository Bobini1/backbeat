use std::path::{Path, PathBuf};

/// Read a file at this path from our test files.
///
/// # Panics
///
/// Panics if the file is not readable.
#[allow(unused)]
pub(crate) fn test_file_read(path: impl AsRef<Path>) -> Vec<u8> {
	let path = test_path(path);

	let err_msg = format!("Failed to read {path:?}");

	std::fs::read(path).expect(&err_msg)
}

/// Append this path to the start of the test_files directory.
#[allow(unused)]
pub(crate) fn test_path(path: impl AsRef<Path>) -> PathBuf {
	let base = ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	let base = base.join("test_files");
	base.join(path)
}
