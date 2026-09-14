#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::ffi::{CStr, CString};
use std::fs;
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;

use backbeat_c_sdk::*;
use backbeat_core::{AssetId, Sha256};
use backbeat_sdk::test_support::store_asset_unchecked;

mod support;

unsafe fn get_asset(store: *const bkb_store, id: AssetId) -> bkb_asset_data {
	let id = CString::new(id.to_string()).unwrap();
	let mut data = MaybeUninit::<bkb_asset_data>::uninit();
	assert_eq!(
		unsafe { bkb_store_get_asset(store, id.as_ptr(), data.as_mut_ptr()) },
		BKB_OK
	);
	unsafe { data.assume_init() }
}

#[test]
fn reads_inline_and_file_assets_inserted_by_rust() {
	let (_temp, _environment, store) = support::new_test_store("bkb_asset_read", 16);
	let inline = b"inline";
	let file = b"this fixture is larger than the inline threshold";
	let inline_id = AssetId(Sha256::checksum_bytes(inline));
	let file_id = AssetId(Sha256::checksum_bytes(file));
	store_asset_unchecked(&store, inline_id, inline).unwrap();
	store_asset_unchecked(&store, file_id, file).unwrap();
	drop(store);

	let mut store = ptr::null_mut();
	assert_eq!(unsafe { bkb_store_open(&mut store) }, BKB_OK);

	let inline_data = unsafe { get_asset(store, inline_id) };
	assert_eq!(inline_data.kind, BKB_ASSET_DATA_BYTES);
	let bytes = unsafe { inline_data.value.bytes };
	assert_eq!(
		unsafe { slice::from_raw_parts(bytes.ptr, bytes.len) },
		inline
	);
	unsafe { bkb_asset_data_free(inline_data) };

	let file_data = unsafe { get_asset(store, file_id) };
	assert_eq!(file_data.kind, BKB_ASSET_DATA_FILE);
	let path = unsafe { file_data.value.file };
	let path = unsafe { CStr::from_ptr(path.ptr) }.to_str().unwrap();
	assert_eq!(fs::read(path).unwrap(), file);
	unsafe { bkb_asset_data_free(file_data) };

	unsafe { bkb_store_free(store) };
}
