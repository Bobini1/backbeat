use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs;
use std::ptr;
use std::slice;

use backbeat_core::{
	AssetId, AssetPath, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};

use super::*;
use crate::error::{BKB_ERR_INVALID_BUNDLE, BKB_ERR_JSON, BKB_ERR_NULL_ARG, BKB_OK};

fn fixture() -> BackbeatFile {
	let mut assets = HashMap::new();
	assets.insert(
		AssetPath::new("audio/song.ogg").unwrap(),
		AssetId(Sha256::checksum_bytes(b"asset")),
	);
	BackbeatFile {
		filename: ChartFilename::new("chart.sm").unwrap(),
		assets,
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"chart data").unwrap(),
	}
}

unsafe fn take_string(value: bkb_string) -> String {
	let result = unsafe { CStr::from_ptr(value.ptr) }
		.to_str()
		.unwrap()
		.to_owned();
	unsafe { crate::functions::bkb_string_free(value) };
	result
}

unsafe fn take_bytes(value: bkb_bytes) -> Vec<u8> {
	let result = if value.len == 0 {
		assert!(value.ptr.is_null());
		Vec::new()
	} else {
		unsafe { slice::from_raw_parts(value.ptr, value.len) }.to_vec()
	};
	unsafe { crate::functions::bkb_bytes_free(value) };
	result
}

#[test]
fn json_roundtrip_and_methods() {
	let expected = fixture();
	let json = expected.to_json();
	let mut bb = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_bb_from_json(json.as_ptr(), json.len(), &mut bb) },
		BKB_OK
	);
	let filename: &[u8] =
		unsafe { slice::from_raw_parts((*bb).filename.ptr.cast(), (*bb).filename.len) };
	assert_eq!(filename, b"chart.sm");
	assert_eq!(unsafe { (*bb).assets_len }, 1);
	let assets = unsafe { slice::from_raw_parts((*bb).assets, (*bb).assets_len) };
	let asset_path: &[u8] =
		unsafe { slice::from_raw_parts(assets[0].path.ptr.cast(), assets[0].path.len) };
	assert_eq!(asset_path, b"audio/song.ogg");
	let desc: &[u8] = unsafe { slice::from_raw_parts((*bb).desc.ptr.cast(), (*bb).desc.len) };
	assert_eq!(desc, b"test");
	let chart = unsafe { slice::from_raw_parts((*bb).chart, (*bb).chart_len) };
	assert_eq!(chart, b"chart data");

	let mut string = bkb_string {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(unsafe { bkb_bb_chart_sha256(bb, &mut string) }, BKB_OK);
	assert_eq!(
		unsafe { take_string(string) },
		expected.chart_sha256().to_string()
	);

	assert_eq!(unsafe { bkb_bb_bundle_id(bb, &mut string) }, BKB_OK);
	assert_eq!(
		unsafe { take_string(string) },
		expected.bundle_id().to_string()
	);

	assert_eq!(
		unsafe { bkb_bb_combined_assets_id(bb, &mut string) },
		BKB_OK
	);
	assert_eq!(
		unsafe { take_string(string) },
		expected.combined_assets_id().to_string()
	);

	let path = CString::new("AUDIO/SONG.OGG").unwrap();
	assert_eq!(
		unsafe { bkb_bb_resolve_path(bb, path.as_ptr(), &mut string) },
		BKB_OK
	);
	assert_eq!(
		unsafe { take_string(string) },
		expected.resolve_path("audio/song.ogg").unwrap().to_string()
	);

	let mut output = bkb_bytes {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(unsafe { bkb_bb_to_json(bb, &mut output) }, BKB_OK);
	let output = unsafe { take_bytes(output) };
	assert_eq!(BackbeatFile::from_json(&output).unwrap(), expected);

	unsafe { bkb_bb_free(bb) };
}

#[test]
fn from_file_loads_a_bb() {
	let root = tempfile::tempdir().unwrap();
	let path = root.path().join("chart.bb");
	fs::write(&path, fixture().to_json()).unwrap();
	let path = CString::new(path.to_string_lossy().as_bytes()).unwrap();
	let mut bb = ptr::null_mut();
	assert_eq!(unsafe { bkb_bb_from_file(path.as_ptr(), &mut bb) }, BKB_OK);
	unsafe { bkb_bb_free(bb) };
}

#[test]
fn zero_byte_chart_fixture_uses_a_null_pointer_and_zero_length() {
	let json = include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../../fixtures/bb/zero-byte-chart.bb"
	));
	let mut bb = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_bb_from_json(json.as_ptr(), json.len(), &mut bb) },
		BKB_OK
	);

	assert!(unsafe { (*bb).chart.is_null() });
	assert_eq!(unsafe { (*bb).chart_len }, 0);

	let mut output = bkb_bytes {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(unsafe { bkb_bb_to_json(bb, &mut output) }, BKB_OK);
	let reparsed = BackbeatFile::from_json(&unsafe { take_bytes(output) }).unwrap();
	assert!(reparsed.chart.decompress().is_empty());

	unsafe { bkb_bb_free(bb) };
}

#[test]
fn invalid_json_and_charts_have_semantic_errors() {
	let invalid_json = b"not json";
	let mut bb = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_bb_from_json(invalid_json.as_ptr(), invalid_json.len(), &mut bb) },
		BKB_ERR_JSON
	);

	let invalid_chart =
		br#"{"filename":"chart.sm","assets":{},"desc":"test","chart":"not base64"}"#;
	assert_eq!(
		unsafe { bkb_bb_from_json(invalid_chart.as_ptr(), invalid_chart.len(), &mut bb) },
		BKB_ERR_INVALID_BUNDLE
	);
}

#[test]
fn null_arguments_are_rejected() {
	let json = fixture().to_json();
	let mut bb = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_bb_from_json(ptr::null(), 0, &mut bb) },
		BKB_ERR_NULL_ARG
	);
	assert_eq!(
		unsafe { bkb_bb_from_json(json.as_ptr(), json.len(), ptr::null_mut()) },
		BKB_ERR_NULL_ARG
	);
	let mut output = bkb_bytes {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(
		unsafe { bkb_bb_to_json(ptr::null(), &mut output) },
		BKB_ERR_NULL_ARG
	);
	unsafe { bkb_bb_free(ptr::null_mut()) };
}
