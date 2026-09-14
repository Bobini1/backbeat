use std::ffi::CStr;

use libsqlite3_sys::{sqlite3_compileoption_used, sqlite3_libversion_number, sqlite3_threadsafe};

use crate::error::{BKB_ERR_INCOMPATIBLE_SQLITE, bkb_error_code};

/// Oldest SQLite release that may be linked with Backbeat.
///
/// The linked SQLite must also be thread-safe, include FTS5, and retain JSON,
/// WAL, foreign-key, and trigger support.
pub const BKB_SQLITE_MIN_VERSION_NUMBER: i32 = 3_038_000;

/// Return the version number of the SQLite implementation linked into the process.
#[unsafe(no_mangle)]
pub extern "C" fn bkb_sqlite_version_number() -> i32 {
	unsafe { sqlite3_libversion_number() }
}

pub(crate) fn ensure_compatible() -> Result<(), bkb_error_code> {
	let compatible = bkb_sqlite_version_number() >= BKB_SQLITE_MIN_VERSION_NUMBER
		&& unsafe { sqlite3_threadsafe() != 0 }
		&& compile_option(c"ENABLE_FTS5")
		&& !compile_option(c"OMIT_JSON")
		&& !compile_option(c"OMIT_WAL")
		&& !compile_option(c"OMIT_FOREIGN_KEY")
		&& !compile_option(c"OMIT_TRIGGER");

	if compatible {
		Ok(())
	} else {
		Err(BKB_ERR_INCOMPATIBLE_SQLITE)
	}
}

fn compile_option(option: &CStr) -> bool {
	unsafe { sqlite3_compileoption_used(option.as_ptr()) != 0 }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn linked_sqlite_is_compatible() {
		assert_eq!(ensure_compatible(), Ok(()));
		assert!(bkb_sqlite_version_number() >= BKB_SQLITE_MIN_VERSION_NUMBER);
	}
}
