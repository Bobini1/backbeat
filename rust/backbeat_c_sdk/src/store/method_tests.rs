use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs;
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;
use std::time::{Duration, Instant};

use backbeat_core::{
	AssetId, AssetPath, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};
use backbeat_sdk::collections::{CollectionDownloadDataFailure, CollectionDownloadDataReport};
use backbeat_sdk::{Backbeat, DataId};

use super::*;
use crate::error::{BKB_ERR_IO, BKB_ERR_NOT_FOUND, BKB_ERR_NULL_ARG, BKB_ERR_PARSE, BKB_OK};
use crate::functions::{bkb_bytes_free, bkb_string_free};
use crate::types::bkb_str;

fn open_store() -> (tempfile::TempDir, *mut bkb_store) {
	let root = tempfile::tempdir().unwrap();
	let config_dir = root.path().join("config");
	let store_dir = root.path().join("store");
	fs::create_dir_all(&config_dir).unwrap();
	fs::write(
		config_dir.join("backbeat.toml"),
		format!("[store]\npath = {:?}\n", store_dir.to_string_lossy()),
	)
	.unwrap();
	let inner = Backbeat::open_with_overridden_config_dir(config_dir).unwrap();
	(root, Box::into_raw(Box::new(bkb_store { inner })))
}

fn fixture() -> BackbeatFile {
	BackbeatFile {
		filename: ChartFilename::new("song.bms").unwrap(),
		assets: HashMap::new(),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#TITLE Test\n#ARTIST Backbeat\n").unwrap(),
	}
}

fn cstring(value: impl AsRef<str>) -> CString {
	CString::new(value.as_ref()).unwrap()
}

unsafe fn take_string(value: bkb_string) -> String {
	let result = unsafe { CStr::from_ptr(value.ptr) }
		.to_str()
		.unwrap()
		.to_owned();
	unsafe { bkb_string_free(value) };
	result
}

unsafe fn take_bytes(value: bkb_bytes) -> Vec<u8> {
	let result = if value.len == 0 {
		assert!(value.ptr.is_null());
		Vec::new()
	} else {
		unsafe { slice::from_raw_parts(value.ptr, value.len) }.to_vec()
	};
	unsafe { bkb_bytes_free(value) };
	result
}

unsafe fn borrowed_bytes(value: bkb_str) -> Vec<u8> {
	if value.len == 0 {
		return Vec::new();
	}
	unsafe { slice::from_raw_parts(value.ptr.cast(), value.len) }.to_vec()
}

#[test]
fn synchronous_store_methods_work() {
	let (root, store) = open_store();
	let bb_value = fixture();
	let expected_bundle_id = bb_value.bundle_id().to_string();
	let chart_id = format!("sha256/{}", bb_value.chart_sha256());
	let bb = bkb_bb::alloc(bb_value);
	let mut string = bkb_string {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(
		unsafe { bkb_store_import_bundle(store, bb, &mut string) },
		BKB_OK
	);
	assert_eq!(unsafe { take_string(string) }, expected_bundle_id);
	unsafe { crate::bb::bkb_bb_free(bb) };
	let extension = cstring("bms");
	let extensions = [extension.as_ptr()];
	let mut search_result = ptr::null_mut();
	assert_eq!(
		unsafe {
			bkb_store_search_bundles(
				store,
				ptr::null(),
				0,
				1,
				extensions.as_ptr(),
				extensions.len(),
				&mut search_result,
			)
		},
		BKB_OK
	);
	let search_result_value = unsafe { &*search_result };
	assert_eq!(search_result_value.total, 1);
	assert_eq!(search_result_value.charts_len, 1);
	assert!(!search_result_value.has_more);
	let chart = unsafe { &*search_result_value.charts };
	assert_eq!(
		unsafe { borrowed_bytes(chart.bundle_id) },
		expected_bundle_id.as_bytes()
	);
	assert_eq!(unsafe { borrowed_bytes(chart.description) }, b"test");
	assert_eq!(unsafe { borrowed_bytes(chart.extension) }, b"bms");
	unsafe { bkb_bundle_search_result_free(search_result) };

	let bundle_id = cstring(&expected_bundle_id);
	let chart_id = cstring(&chart_id);
	assert_eq!(
		unsafe { bkb_store_bundle_download_assets(store, bundle_id.as_ptr()) },
		BKB_OK
	);

	let mut found = false;
	assert_eq!(
		unsafe { bkb_store_has_bundle(store, bundle_id.as_ptr(), &mut found) },
		BKB_OK
	);
	assert!(found);
	assert_eq!(
		unsafe { bkb_store_has_chart(store, chart_id.as_ptr(), &mut found) },
		BKB_OK
	);
	assert!(found);

	let mut bytes = bkb_bytes {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(
		unsafe { bkb_store_get_chart_data(store, chart_id.as_ptr(), &mut bytes) },
		BKB_OK
	);
	assert_eq!(
		unsafe { take_bytes(bytes) },
		b"#TITLE Test\n#ARTIST Backbeat\n"
	);

	let mut returned_bb = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_get_chart(store, chart_id.as_ptr(), &mut returned_bb) },
		BKB_OK
	);
	unsafe { crate::bb::bkb_bb_free(returned_bb) };
	assert_eq!(
		unsafe { bkb_store_get_bundle(store, bundle_id.as_ptr(), &mut returned_bb) },
		BKB_OK
	);
	unsafe { crate::bb::bkb_bb_free(returned_bb) };
	let chart_output = root.path().join("chart");
	let chart_output_arg = cstring(chart_output.to_string_lossy());
	assert_eq!(
		unsafe {
			bkb_store_export_chart(store, chart_id.as_ptr(), chart_output_arg.as_ptr(), false)
		},
		BKB_OK
	);
	assert_eq!(
		fs::read(chart_output.join("song.bms")).unwrap(),
		b"#TITLE Test\n#ARTIST Backbeat\n"
	);
	let bundle_output = root.path().join("bundle.bbzip");
	let bundle_output_arg = cstring(bundle_output.to_string_lossy());
	assert_eq!(
		unsafe {
			bkb_store_export_bundle(store, bundle_id.as_ptr(), bundle_output_arg.as_ptr(), true)
		},
		BKB_OK
	);
	assert!(bundle_output.is_file());

	let asset_data = b"asset bytes";
	let asset_id = AssetId(Sha256::checksum_bytes(asset_data)).to_string();
	let asset_file = root.path().join("asset.bin");
	fs::write(&asset_file, asset_data).unwrap();
	let asset_file = cstring(asset_file.to_string_lossy());
	assert_eq!(
		unsafe { bkb_store_import_asset(store, asset_file.as_ptr()) },
		BKB_OK
	);
	let asset_id = cstring(asset_id);
	assert_eq!(
		unsafe { bkb_store_has_asset(store, asset_id.as_ptr(), &mut found) },
		BKB_OK
	);
	assert!(found);
	let mut returned_asset = MaybeUninit::<bkb_asset_data>::uninit();
	assert_eq!(
		unsafe { bkb_store_get_asset(store, asset_id.as_ptr(), returned_asset.as_mut_ptr()) },
		BKB_OK
	);
	let returned_asset = unsafe { returned_asset.assume_init() };
	assert_eq!(returned_asset.kind, BKB_ASSET_DATA_BYTES);
	assert_eq!(
		unsafe { take_bytes(returned_asset.value.bytes) },
		asset_data
	);
	let file_data = vec![7; 128 * 1024];
	let file_asset_id = AssetId(Sha256::checksum_bytes(&file_data));
	let stored_file = root
		.path()
		.join("store")
		.join(".assets")
		.join(file_asset_id.fanned_path());
	fs::create_dir_all(stored_file.parent().unwrap()).unwrap();
	fs::write(&stored_file, &file_data).unwrap();
	let file_asset_id = cstring(file_asset_id.to_string());
	let mut returned_file = MaybeUninit::<bkb_asset_data>::uninit();
	assert_eq!(
		unsafe { bkb_store_get_asset(store, file_asset_id.as_ptr(), returned_file.as_mut_ptr()) },
		BKB_OK
	);
	let returned_asset = unsafe { returned_file.assume_init() };
	assert_eq!(returned_asset.kind, BKB_ASSET_DATA_FILE);
	let returned_path = unsafe { take_string(returned_asset.value.file) };
	assert_eq!(returned_path, stored_file.to_string_lossy());
	assert_eq!(fs::read(returned_path).unwrap(), file_data);
	let mut owned_asset = MaybeUninit::<bkb_asset_data>::uninit();
	assert_eq!(
		unsafe { bkb_store_get_asset(store, asset_id.as_ptr(), owned_asset.as_mut_ptr()) },
		BKB_OK
	);
	unsafe { bkb_asset_data_free(owned_asset.assume_init()) };
	assert_eq!(
		unsafe { bkb_store_get_asset(store, file_asset_id.as_ptr(), owned_asset.as_mut_ptr()) },
		BKB_OK
	);
	unsafe { bkb_asset_data_free(owned_asset.assume_init()) };

	let mut stats = bkb_stats {
		charts: 0,
		tables: 0,
		courses: 0,
		packs: 0,
		asset_count: 0,
		asset_bytes: 0,
		db_bytes: 0,
	};
	assert_eq!(unsafe { bkb_store_stats(store, &mut stats) }, BKB_OK);
	assert_eq!(stats.charts, 1);
	assert_eq!(stats.tables, 0);
	assert_eq!(stats.courses, 0);
	assert_eq!(stats.packs, 0);
	assert_eq!(stats.asset_count, 1);
	assert_eq!(
		unsafe { bkb_store_bundle_rm(store, bundle_id.as_ptr()) },
		BKB_OK
	);
	assert_eq!(
		unsafe { bkb_store_has_bundle(store, bundle_id.as_ptr(), &mut found) },
		BKB_OK
	);
	assert!(!found);
	assert_eq!(
		unsafe { bkb_store_has_chart(store, chart_id.as_ptr(), &mut found) },
		BKB_OK
	);
	assert!(!found);
	assert_eq!(
		unsafe { bkb_store_bundle_rm(store, bundle_id.as_ptr()) },
		BKB_ERR_NOT_FOUND
	);

	unsafe { crate::store::bkb_store_free(store) };
}

#[test]
fn zero_byte_chart_roundtrips_through_store_methods() {
	let (_root, store) = open_store();
	let json = include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../../fixtures/bb/zero-byte-chart.bb"
	));
	let bb_value = BackbeatFile::from_json(json).unwrap();
	let bundle_id = cstring(bb_value.bundle_id().to_string());
	let chart_id = cstring(format!("sha256/{}", bb_value.chart_sha256()));
	let bb = bkb_bb::alloc(bb_value);
	let mut imported_id = bkb_string {
		ptr: ptr::null_mut(),
		len: 0,
	};

	assert_eq!(
		unsafe { bkb_store_import_bundle(store, bb, &mut imported_id) },
		BKB_OK
	);
	unsafe { bkb_string_free(imported_id) };
	unsafe { crate::bb::bkb_bb_free(bb) };

	let mut bytes = bkb_bytes {
		ptr: ptr::dangling_mut(),
		len: usize::MAX,
	};
	assert_eq!(
		unsafe { bkb_store_get_chart_data(store, chart_id.as_ptr(), &mut bytes) },
		BKB_OK
	);
	assert!(unsafe { take_bytes(bytes) }.is_empty());

	let mut returned = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_get_chart(store, chart_id.as_ptr(), &mut returned) },
		BKB_OK
	);
	assert!(unsafe { (*returned).chart.is_null() });
	assert_eq!(unsafe { (*returned).chart_len }, 0);
	unsafe { crate::bb::bkb_bb_free(returned) };

	assert_eq!(
		unsafe { bkb_store_get_bundle(store, bundle_id.as_ptr(), &mut returned) },
		BKB_OK
	);
	assert!(unsafe { (*returned).chart.is_null() });
	assert_eq!(unsafe { (*returned).chart_len }, 0);
	unsafe { crate::bb::bkb_bb_free(returned) };

	unsafe { bkb_store_free(store) };
}

#[test]
fn resolves_bundle_asset_paths() {
	let (root, store) = open_store();
	let asset_data = b"resolved asset";
	let asset_id = AssetId(Sha256::checksum_bytes(asset_data));
	let asset_file = root.path().join("resolved.bin");
	fs::write(&asset_file, asset_data).unwrap();
	let asset_file = cstring(asset_file.to_string_lossy());
	assert_eq!(
		unsafe { bkb_store_import_asset(store, asset_file.as_ptr()) },
		BKB_OK
	);

	let bundle = BackbeatFile {
		filename: ChartFilename::new("resolve.bms").unwrap(),
		assets: HashMap::from([(AssetPath::from_path("assets/test.bin").unwrap(), asset_id)]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#TITLE Resolve\n").unwrap(),
	};
	let bundle_id = cstring(bundle.bundle_id().to_string());
	let bb = bkb_bb::alloc(bundle);
	let mut imported_bundle_id = bkb_string {
		ptr: ptr::null_mut(),
		len: 0,
	};
	assert_eq!(
		unsafe { bkb_store_import_bundle(store, bb, &mut imported_bundle_id) },
		BKB_OK
	);
	unsafe { crate::bb::bkb_bb_free(bb) };
	unsafe { bkb_string_free(imported_bundle_id) };

	let asset_path = cstring("ASSETS/TEST.BIN");
	let mut resolved_asset = MaybeUninit::<bkb_asset_data>::uninit();
	assert_eq!(
		unsafe {
			bkb_store_resolve_path(
				store,
				bundle_id.as_ptr(),
				asset_path.as_ptr(),
				resolved_asset.as_mut_ptr(),
			)
		},
		BKB_OK
	);
	let resolved_asset = unsafe { resolved_asset.assume_init() };
	assert_eq!(resolved_asset.kind, BKB_ASSET_DATA_BYTES);
	assert_eq!(
		unsafe { take_bytes(resolved_asset.value.bytes) },
		asset_data
	);

	unsafe { crate::store::bkb_store_free(store) };
}

#[test]
fn download_and_collection_queries_work_on_an_empty_store() {
	let (_root, store) = open_store();
	let mut snapshots = MaybeUninit::<bkb_download_snapshot_list>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_all_progress(store, snapshots.as_mut_ptr()) },
		BKB_OK
	);
	let snapshots = unsafe { snapshots.assume_init() };
	assert!(snapshots.items.is_null());
	assert_eq!(snapshots.items_len, 0);
	unsafe { bkb_download_snapshot_list_free(snapshots) };

	let mut overview = MaybeUninit::<bkb_download_overview>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_overview(store, overview.as_mut_ptr()) },
		BKB_OK
	);
	let overview = unsafe { overview.assume_init() };
	assert_eq!(overview.total, 0);
	assert!(overview.first_error.ptr.is_null());
	unsafe { bkb_download_overview_free(overview) };

	let mut list = MaybeUninit::<bkb_download_list_result>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_list(store, 0, 20, list.as_mut_ptr()) },
		BKB_OK
	);
	let list = unsafe { list.assume_init() };
	assert_eq!(list.total, 0);
	assert!(list.downloads.is_null());
	assert_eq!(list.downloads_len, 0);
	assert!(!list.has_more);
	unsafe { bkb_download_list_result_free(list) };

	let unknown_asset = AssetId(Sha256::checksum_bytes(b"unknown")).to_string();
	let unknown_asset = cstring(unknown_asset);
	let data_id = bkb_data_id {
		kind: BKB_DATA_ASSET,
		value: unknown_asset.as_ptr(),
	};
	let mut progress = MaybeUninit::<bkb_optional_download_progress>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_progress(store, &data_id, progress.as_mut_ptr()) },
		BKB_OK
	);
	assert!(!unsafe { progress.assume_init() }.is_some);
	let mut count = usize::MAX;
	assert_eq!(
		unsafe { bkb_store_download_clear_finished(store, &mut count) },
		BKB_OK
	);
	assert_eq!(count, 0);
	let mut cancelled = true;
	assert_eq!(
		unsafe { bkb_store_download_cancel(store, &data_id, &mut cancelled) },
		BKB_OK
	);
	assert!(!cancelled);
	assert_eq!(
		unsafe { bkb_store_download_asset_cancel(store, unknown_asset.as_ptr(), &mut cancelled) },
		BKB_OK
	);
	assert!(!cancelled);

	let url = cstring("https://collections.example/test");
	let mut found = true;
	for has in [
		bkb_store_has_collection,
		bkb_store_has_table,
		bkb_store_has_course,
		bkb_store_has_pack,
	] {
		assert_eq!(unsafe { has(store, url.as_ptr(), &mut found) }, BKB_OK);
		assert!(!found);
	}
	let mut kind = 0;
	assert_eq!(
		unsafe { bkb_store_collection_rm(store, url.as_ptr(), false, &mut kind) },
		BKB_ERR_NOT_FOUND
	);
	for list in [
		bkb_store_list_tables,
		bkb_store_list_courses,
		bkb_store_list_packs,
	] {
		let mut values = ptr::null_mut();
		assert_eq!(unsafe { list(store, ptr::null(), 0, &mut values) }, BKB_OK);
		assert_eq!(unsafe { (*values).items_len }, 0);
		unsafe { bkb_collection_metadata_list_free(values) };

		let gamemode = cstring("kshoot");
		let gamemodes = [gamemode.as_ptr()];
		let mut values = ptr::null_mut();
		assert_eq!(
			unsafe { list(store, gamemodes.as_ptr(), gamemodes.len(), &mut values) },
			BKB_OK
		);
		assert_eq!(unsafe { (*values).items_len }, 0);
		unsafe { bkb_collection_metadata_list_free(values) };

		assert_eq!(
			unsafe { list(store, ptr::null(), 1, &mut values) },
			BKB_ERR_NULL_ARG
		);
	}
	let mut pack = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_get_pack(store, url.as_ptr(), &mut pack) },
		BKB_ERR_NOT_FOUND
	);
	let mut table = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_get_table(store, url.as_ptr(), &mut table) },
		BKB_ERR_NOT_FOUND
	);
	let mut course = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_get_course(store, url.as_ptr(), &mut course) },
		BKB_ERR_NOT_FOUND
	);

	let missing = cstring("missing.bbzip");
	assert_eq!(
		unsafe { bkb_store_import_bbzip(store, missing.as_ptr()) },
		BKB_ERR_IO
	);
	unsafe { crate::store::bkb_store_free(store) };
}

#[test]
fn identifiers_and_outputs_are_validated() {
	let invalid = cstring("invalid");
	let mut found = false;
	assert_eq!(
		unsafe { bkb_store_has_asset(ptr::null(), invalid.as_ptr(), &mut found) },
		BKB_ERR_PARSE
	);
	assert_eq!(
		unsafe { bkb_store_has_asset(ptr::null(), invalid.as_ptr(), ptr::null_mut()) },
		BKB_ERR_NULL_ARG
	);
}

#[test]
fn maintenance_methods_return_owned_reports_and_counts() {
	let (_root, store) = open_store();
	let mut report = ptr::null_mut();
	assert_eq!(
		unsafe { bkb_store_corruption_check(store, &mut report) },
		BKB_OK
	);
	let report_value = unsafe { &*report };
	assert!(report_value.ok);
	assert_eq!(report_value.issue_count, 0);
	assert_eq!(report_value.large_asset_count, 0);
	assert_eq!(report_value.chart_count, 0);
	assert_eq!(report_value.chart_id_count, 0);
	assert_eq!(report_value.missing_assets_len, 0);
	assert_eq!(report_value.corrupt_assets_len, 0);
	assert_eq!(report_value.corrupt_charts_len, 0);
	assert_eq!(report_value.wrong_chart_ids_len, 0);
	assert_eq!(report_value.uncomputable_chart_ids_len, 0);
	assert_eq!(report_value.dangling_asset_refs_len, 0);
	unsafe { bkb_corruption_report_free(report) };

	let mut count = u64::MAX;
	assert_eq!(
		unsafe { bkb_store_asset_prune(store, true, &mut count) },
		BKB_OK
	);
	assert_eq!(count, 0);
	assert_eq!(
		unsafe { bkb_store_disk_prune(store, true, &mut count) },
		BKB_OK
	);
	assert_eq!(count, 0);
	assert_eq!(unsafe { bkb_store_corruption_repair(store) }, BKB_OK);
	unsafe { bkb_store_free(store) };
}

#[test]
fn store_should_refresh_validates_outputs_and_returns_revision() {
	let (_root, store) = open_store();
	let mut should_refresh = false;
	let mut revision = -1;
	assert_eq!(
		unsafe { bkb_store_should_refresh(store, -1, &mut should_refresh, &mut revision) },
		BKB_OK
	);
	assert!(should_refresh);
	assert_eq!(revision, 0);
	assert_eq!(
		unsafe { bkb_store_should_refresh(store, revision, &mut should_refresh, &mut revision) },
		BKB_OK
	);
	assert!(!should_refresh);
	assert_eq!(
		unsafe { bkb_store_should_refresh(store, revision, ptr::null_mut(), &mut revision) },
		BKB_ERR_NULL_ARG
	);
	assert_eq!(
		unsafe { bkb_store_should_refresh(store, revision, &mut should_refresh, ptr::null_mut()) },
		BKB_ERR_NULL_ARG
	);
	unsafe { bkb_store_free(store) };
}

#[test]
fn server_download_functions_return_immediately_and_report_progress() {
	let (_root, store) = open_store();
	let asset_id = AssetId(Sha256::checksum_bytes(b"missing asset")).to_string();
	let bundle_id = fixture().bundle_id().to_string();
	let chart_id = format!("sha256/{}", Sha256::checksum_bytes(b"missing chart"));
	let asset_id_arg = cstring(&asset_id);
	let bundle_id_arg = cstring(&bundle_id);
	let chart_id_arg = cstring(&chart_id);
	assert_eq!(
		unsafe { bkb_store_server_download_asset(store, asset_id_arg.as_ptr()) },
		BKB_OK
	);
	assert_eq!(
		unsafe { bkb_store_server_download_bundle(store, bundle_id_arg.as_ptr()) },
		BKB_OK
	);
	assert_eq!(
		unsafe { bkb_store_server_download_chart(store, chart_id_arg.as_ptr()) },
		BKB_OK
	);

	let deadline = Instant::now() + Duration::from_secs(5);
	let data_ids = [
		bkb_data_id {
			kind: BKB_DATA_ASSET,
			value: asset_id_arg.as_ptr(),
		},
		bkb_data_id {
			kind: BKB_DATA_BUNDLE,
			value: bundle_id_arg.as_ptr(),
		},
		bkb_data_id {
			kind: BKB_DATA_CHART,
			value: chart_id_arg.as_ptr(),
		},
	];
	loop {
		let mut failed = 0;
		for data_id in &data_ids {
			let mut progress = MaybeUninit::<bkb_optional_download_progress>::uninit();
			assert_eq!(
				unsafe { bkb_store_download_progress(store, data_id, progress.as_mut_ptr()) },
				BKB_OK
			);
			let progress = unsafe { progress.assume_init() };
			if progress.is_some && progress.value.state == BKB_DOWNLOAD_FAILED {
				failed += 1;
			}
		}
		if failed == data_ids.len() {
			break;
		}
		assert!(Instant::now() < deadline, "downloads did not finish");
		std::thread::sleep(Duration::from_millis(10));
	}

	let mut snapshots = MaybeUninit::<bkb_download_snapshot_list>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_all_progress(store, snapshots.as_mut_ptr()) },
		BKB_OK
	);
	let snapshots = unsafe { snapshots.assume_init() };
	assert_eq!(snapshots.items_len, 3);
	let snapshot_values = unsafe { slice::from_raw_parts(snapshots.items, snapshots.items_len) };
	assert!(snapshot_values.iter().all(|value| {
		value.progress.state == BKB_DOWNLOAD_FAILED && !value.error.ptr.is_null()
	}));
	unsafe { bkb_download_snapshot_list_free(snapshots) };

	let mut overview = MaybeUninit::<bkb_download_overview>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_overview(store, overview.as_mut_ptr()) },
		BKB_OK
	);
	let overview = unsafe { overview.assume_init() };
	assert_eq!(overview.total, 3);
	assert_eq!(overview.failed, 3);
	assert!(!overview.first_error.ptr.is_null());
	unsafe { bkb_download_overview_free(overview) };

	let mut list = MaybeUninit::<bkb_download_list_result>::uninit();
	assert_eq!(
		unsafe { bkb_store_download_list(store, 0, 2, list.as_mut_ptr()) },
		BKB_OK
	);
	let list = unsafe { list.assume_init() };
	assert_eq!(list.total, 3);
	assert_eq!(list.failed, 3);
	assert_eq!(list.downloads_len, 2);
	assert!(list.has_more);
	unsafe { bkb_download_list_result_free(list) };

	let mut list = MaybeUninit::<bkb_download_list_result>::uninit();
	assert_eq!(
		unsafe { bkb_store_collection_download_list(store, 0, 20, list.as_mut_ptr()) },
		BKB_OK
	);
	let list = unsafe { list.assume_init() };
	assert_eq!(list.total, 2);
	assert_eq!(list.failed, 2);
	assert_eq!(list.downloads_len, 2);
	assert!(!list.has_more);
	unsafe { bkb_download_list_result_free(list) };

	unsafe { bkb_store_free(store) };
}

#[test]
fn collection_download_report_has_owned_failures() {
	let asset_id = AssetId(Sha256::checksum_bytes(b"failed asset"));
	let expected_asset_id = asset_id.to_string();
	let report = util::collection_download_data_report(CollectionDownloadDataReport {
		assets_failed: 1,
		errors: vec![CollectionDownloadDataFailure {
			item: DataId::Asset(asset_id),
			message: "download failed".to_owned(),
		}],
		..CollectionDownloadDataReport::default()
	})
	.unwrap();

	assert_eq!(report.assets_failed, 1);
	assert_eq!(report.errors_len, 1);
	let failure = unsafe { &*report.errors };
	assert_eq!(failure.item.kind, BKB_DATA_ASSET);
	assert_eq!(
		unsafe { CStr::from_ptr(failure.item.value.ptr) }.to_str(),
		Ok(expected_asset_id.as_str())
	);
	assert_eq!(
		unsafe { CStr::from_ptr(failure.message.ptr) }.to_str(),
		Ok("download failed")
	);
	unsafe { bkb_collection_download_data_report_free(report) };
}
