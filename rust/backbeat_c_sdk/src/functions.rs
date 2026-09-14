use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use crate::error::{bkb_error_code, error_string};
use crate::types::{bkb_bytes, bkb_string};

#[unsafe(no_mangle)]
pub extern "C" fn bkb_error_string(code: bkb_error_code) -> *const c_char {
	error_string(code).as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_string_free(value: bkb_string) {
	if !value.ptr.is_null() {
		unsafe { drop(CString::from_raw(value.ptr)) };
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bytes_free(value: bkb_bytes) {
	if !value.ptr.is_null() {
		let bytes = ptr::slice_from_raw_parts_mut(value.ptr, value.len);
		unsafe { drop(Box::from_raw(bytes)) };
	}
}

#[cfg(test)]
mod tests;
