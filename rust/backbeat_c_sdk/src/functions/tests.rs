use std::ffi::CStr;
use std::ptr;

use super::*;
use crate::error::{BKB_ERR_INVALID_STRING, BKB_OK};

#[test]
fn error_strings_are_nul_terminated() {
	let success = unsafe { CStr::from_ptr(bkb_error_string(BKB_OK)) };
	let unknown = unsafe { CStr::from_ptr(bkb_error_string(i32::MAX)) };
	assert_eq!(success.to_bytes(), b"success");
	assert_eq!(unknown.to_bytes(), b"unknown error");
}

#[test]
fn strings_include_the_length_without_the_terminator() {
	let value = bkb_string::new("backbeat".into()).unwrap();
	assert_eq!(value.len, 8);
	assert_eq!(unsafe { CStr::from_ptr(value.ptr) }.to_bytes(), b"backbeat");
	unsafe { bkb_string_free(value) };
}

#[test]
fn strings_reject_interior_nuls() {
	let result = bkb_string::new("back\0beat".into());
	assert!(matches!(result, Err(BKB_ERR_INVALID_STRING)));
}

#[test]
fn empty_owned_bytes_use_a_null_pointer() {
	let value = bkb_bytes::new(Vec::new());
	assert!(value.ptr.is_null());
	assert_eq!(value.len, 0);
	unsafe { bkb_bytes_free(value) };
}

#[test]
fn free_functions_accept_null_values() {
	unsafe {
		bkb_string_free(bkb_string {
			ptr: ptr::null_mut(),
			len: 0,
		});
		bkb_bytes_free(bkb_bytes {
			ptr: ptr::null_mut(),
			len: 0,
		});
	}
}
