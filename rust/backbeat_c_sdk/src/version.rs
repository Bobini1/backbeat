use std::os::raw::c_char;

pub const BKB_LIBVERSION: i32 = 1;

const LIBCOMMIT_HASH: &[u8] = concat!(env!("BKB_LIBCOMMIT_HASH"), "\0").as_bytes();

/// Return the library API version.
#[unsafe(no_mangle)]
pub extern "C" fn bkb_libversion() -> i32 {
	BKB_LIBVERSION
}

/// Return the Git commit used to build the library.
///
/// The returned string is static and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn bkb_libcommithash() -> *const c_char {
	LIBCOMMIT_HASH.as_ptr().cast()
}

#[cfg(test)]
mod tests {
	use std::ffi::CStr;

	use super::*;

	#[test]
	fn runtime_identifiers_match_the_build() {
		let commit_hash = unsafe { CStr::from_ptr(bkb_libcommithash()) };
		assert_eq!(bkb_libversion(), BKB_LIBVERSION);
		assert_eq!(
			commit_hash.to_bytes(),
			env!("BKB_LIBCOMMIT_HASH").as_bytes()
		);
	}
}
