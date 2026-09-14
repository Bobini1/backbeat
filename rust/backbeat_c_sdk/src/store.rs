use std::os::raw::c_char;
use std::ptr;

use backbeat_core::{CollectionHeader, ValidGamemodeIdentifier};
use backbeat_sdk::Backbeat;
use backbeat_sdk::assets::AssetData;
use backbeat_sdk::collections::{CollectionUpsertResult, CollectionUpsertStatus};
use backbeat_sdk::store::stats::StoreStats;

mod ffi_types;
pub(crate) mod util;

pub use ffi_types::*;

use self::util::{
	collection_download_data_report, collection_kind, download_list_result, download_overview,
	download_snapshot_list, has_collection_kind, optional_download_progress, parse_asset_id,
	parse_bundle_id, parse_chart_id, parse_data, path_string, run_store, server_config, with_store,
	with_store_async, with_store_spawn,
};
use crate::error::{BKB_ERR_NULL_ARG, bkb_error_code};
use crate::functions::bkb_string_free;
use crate::types::{
	bkb_bb, bkb_bundle_search_result, bkb_bytes, bkb_collection_metadata_list, bkb_course,
	bkb_pack, bkb_server_config, bkb_store, bkb_string, bkb_table, bkb_timestamp, cstr_to_str,
};
use crate::util::run_ffi;

unsafe fn parse_gamemodes(
	values: *const *const c_char,
	len: usize,
) -> Result<Vec<ValidGamemodeIdentifier>, bkb_error_code> {
	if len == 0 {
		return Ok(Vec::new());
	}
	if values.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	unsafe { std::slice::from_raw_parts(values, len) }
		.iter()
		.map(|value| {
			ValidGamemodeIdentifier::new(unsafe { cstr_to_str(*value) }?)
				.map_err(|_| crate::error::BKB_ERR_PARSE)
		})
		.collect()
}

fn collection_header(value: CollectionHeader) -> bkb_collection_header {
	bkb_collection_header {
		last_modified: bkb_timestamp {
			seconds: value.timestamp.timestamp(),
			nanoseconds: value.timestamp.timestamp_subsec_nanos(),
		},
		kind: collection_kind(value.kind),
	}
}

fn collection_upsert_status(value: CollectionUpsertStatus) -> bkb_collection_upsert_status {
	match value {
		CollectionUpsertStatus::Inserted => BKB_COLLECTION_UPSERT_INSERTED,
		CollectionUpsertStatus::Updated => BKB_COLLECTION_UPSERT_UPDATED,
		CollectionUpsertStatus::TimestampUnchanged => BKB_COLLECTION_UPSERT_TIMESTAMP_UNCHANGED,
	}
}

fn collection_upsert_result(value: CollectionUpsertResult) -> bkb_collection_upsert_result {
	bkb_collection_upsert_result {
		kind: collection_kind(value.kind),
		status: collection_upsert_status(value.status),
	}
}

// n.b. bkb_store_open with args is deliberately not implemented - that is still an internal only thing
// and users DO NOT NEED IT.

/// Open the backbeat store. This will create it if it does not yet exist.
///
/// The final program must provide one SQLite implementation for Backbeat and
/// every other native consumer that accesses the live database. Do not call
/// sqlite3_shutdown while a Backbeat store is alive.
///
/// # Safety
///
/// out_store must be non-null and point to writable storage for one bkb_store pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_open(out_store: *mut *mut bkb_store) -> bkb_error_code {
	run_ffi(|| {
		if out_store.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		crate::sqlite::ensure_compatible()?;
		let inner = run_store(Backbeat::open)?;
		let store = Box::into_raw(Box::new(bkb_store { inner }));
		unsafe { out_store.write(store) };
		Ok(())
	})
}

/// Close a backbeat store and release its resources.
///
/// # Safety
///
/// store must be null or a pointer returned by bkb_store_open that has not already been freed. After this call, the pointer must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_free(store: *mut bkb_store) {
	if !store.is_null() {
		unsafe { drop(Box::from_raw(store)) };
	}
}

/// Return whether the store changed since last_revision, along with its current revision.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_should_refresh(
	store: *const bkb_store,
	last_revision: i64,
	out_should_refresh: *mut bool,
	out_revision: *mut i64,
) -> bkb_error_code {
	run_ffi(|| {
		if out_should_refresh.is_null() || out_revision.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let (should_refresh, revision) =
			unsafe { with_store(store, |store| store.should_refresh(last_revision)) }?;
		unsafe { out_should_refresh.write(should_refresh) };
		unsafe { out_revision.write(revision) };
		Ok(())
	})
}

/// Where the user has configured their store to be.
///
/// Note that you probably do not need to care about this, much less do anything with this information.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_dir(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.store_dir().to_path_buf())) }?;
		let value = path_string(value)?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// The path to the logs dir. This is where backbeat logs get written to.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_logs_dir(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.logs_dir())) }?;
		let value = path_string(value)?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// The path to the config dir. This contains a backbeat.toml file.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_config_dir(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.config_dir().to_path_buf())) }?;
		let value = path_string(value)?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// Add a data server to the config and live server list.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// server must be non-null and point to a valid bkb_server_config whose URL is a valid UTF-8 C string for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_server_add(
	store: *const bkb_store,
	server: *const bkb_server_config,
) -> bkb_error_code {
	run_ffi(|| {
		let server = unsafe { server_config(server) }?;
		unsafe { with_store(store, |store| store.server_add(&server)) }?;
		Ok(())
	})
}

/// Remove a data server from the config and live server list.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// server must be non-null and point to a valid bkb_server_config whose URL is a valid UTF-8 C string for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_server_rm(
	store: *const bkb_store,
	server: *const bkb_server_config,
) -> bkb_error_code {
	run_ffi(|| {
		let server = unsafe { server_config(server) }?;
		unsafe { with_store(store, |store| store.server_rm(&server)) }?;
		Ok(())
	})
}

/// Does this store have absolutely zero data servers set up?
///
/// This is probably the weirdest public backbeat function, but it is useful to quickly indicate that this user has literally no way of installing content.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_zero_data_servers(
	store: *const bkb_store,
	out: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.has_zero_data_servers())) }?;
		unsafe { out.write(value) };
		Ok(())
	})
}

/// Get the SQLite connection URL for attaching the store read-only.
///
/// This returns file:///path/to/backbeat.db?mode=ro.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_sqlite_connection_url(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.sqlite_connection_url())) }?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// Get a SQL statement that will attach the backbeat database read-only as backbeat.
///
/// Evaluate this string in sqlite to get a database backbeat attached to your database.
///
/// More specifically, this function returns ATTACH DATABASE 'path_to_backbeat.db' AS backbeat;.
///
/// Execute it on a SQLite connection opened with URI filename handling, such
/// as SQLITE_OPEN_URI or a build configured with SQLITE_USE_URI.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_sqlite_attach_command(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.sqlite_attach_command())) }?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// Get a SQL statement that will detach backbeat from your database.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_sqlite_detach_command(
	store: *const bkb_store,
	out: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.sqlite_detach_command())) }?;
		unsafe { bkb_string::write(out, value) }
	})
}

/// Add a backbeat file to the store. This does not queue up any downloads. Call bkb_store_bundle_download_assets after this to queue up downloads.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// bb must be non-null and point to a valid bkb_bb whose borrowed fields remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_import_bundle(
	store: *const bkb_store,
	bb: *const bkb_bb,
	out_bundle_id: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		if bb.is_null() || out_bundle_id.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let id = unsafe { with_store(store, |store| store.import_bundle((*bb).inner())) }?;
		unsafe { bkb_string::write(out_bundle_id, id.to_string()) }
	})
}

/// Import a .bbzip file into your store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_import_bbzip(
	store: *const bkb_store,
	path: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let path = unsafe { cstr_to_str(path) }?;
		unsafe { with_store(store, |store| store.import_bbzip(path)) }?;
		Ok(())
	})
}

/// Take a path and add it to the store as an asset.
///
/// This API is honestly just provided to flesh out the API. It is very rare you will need this, but if this did not exist, you would have to make a temporary .bbzip or something.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_import_asset(
	store: *const bkb_store,
	path: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let path = unsafe { cstr_to_str(path) }?;
		unsafe { with_store(store, |store| store.import_asset(path)) }?;
		Ok(())
	})
}

/// Export a chart and all its assets to a folder or .bbzip file.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// The ID and output path pointers must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_export_chart(
	store: *const bkb_store,
	chart_id: *const c_char,
	output: *const c_char,
	bbzip: bool,
) -> bkb_error_code {
	run_ffi(|| {
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		let output = unsafe { cstr_to_str(output) }?;
		unsafe { with_store(store, |store| store.export_chart(&chart_id, output, bbzip)) }?;
		Ok(())
	})
}

/// Export a bundle and all its assets to a folder or .bbzip file.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// The ID and output path pointers must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_export_bundle(
	store: *const bkb_store,
	bundle_id: *const c_char,
	output: *const c_char,
	bbzip: bool,
) -> bkb_error_code {
	run_ffi(|| {
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		let output = unsafe { cstr_to_str(output) }?;
		unsafe { with_store(store, |store| store.export_bundle(bundle_id, output, bbzip)) }?;
		Ok(())
	})
}

/// Queue up downloads for all missing assets in this bundle. This bundle has to be installed in your store.
///
/// This is a useful function for repairing bundles with incomplete assets, or bundles that have just been manually added with bkb_store_import_bundle.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_bundle_download_assets(
	store: *const bkb_store,
	bundle_id: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		unsafe { with_store(store, |store| store.bundle_download_assets(bundle_id)) }?;
		Ok(())
	})
}

/// Download an asset from your configured data servers and install it into the store.
///
/// No-op if the asset is already present locally. Concurrent calls for the same asset are coalesced onto a single network fetch by the download manager.
///
/// This starts the download and returns immediately. Completion is reported through store events.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_server_download_asset(
	store: *const bkb_store,
	asset_id: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let asset_id = unsafe { parse_asset_id(asset_id) }?;
		unsafe {
			with_store_spawn(store, |store| async move {
				store.server_download_asset(asset_id).await
			})
		}
	})
}

/// Download a bundle from your configured data servers and install it.
/// This will also download all of the assets referenced by this chart, too.
///
/// No-op if the asset is already present locally. Concurrent calls for the same bundle are coalesced onto a single network fetch by the download manager.
///
/// This starts the download and returns immediately. Completion is reported through store events.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_server_download_bundle(
	store: *const bkb_store,
	bundle_id: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		unsafe {
			with_store_spawn(store, |store| async move {
				store.server_download_bundle(bundle_id).await
			})
		}
	})
}

/// Download a chart from your configured data servers and install it.
/// This will also download all of the assets referenced by this bundle, too.
///
/// No-op if the chart is already present locally. Concurrent calls for the same chart are coalesced onto a single network fetch by the download manager.
///
/// This starts the download and returns immediately. Completion is reported through store events.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_server_download_chart(
	store: *const bkb_store,
	chart_id: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		unsafe {
			with_store_spawn(store, |store| async move {
				store.server_download_chart(&chart_id).await
			})
		}
	})
}

/// Download charts, bundles, and assets referenced by an installed collection.
///
/// The collection at url must already be stored locally.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_collection_fetch_download_data(
	store: *const bkb_store,
	url: *const c_char,
	out_report: *mut bkb_collection_download_data_report,
) -> bkb_error_code {
	run_ffi(|| {
		if out_report.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?.to_owned();
		let report = unsafe {
			with_store_async(store, |store| async move {
				store.collection_fetch_download_data(&url, |_| {}).await
			})
		}?;
		let report = collection_download_data_report(report)?;
		unsafe { out_report.write(report) };
		Ok(())
	})
}

/// Fetch only the header for this collection. Doubles up as a way of checking whether a URL is a valid collection.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_collection_fetch_header(
	store: *const bkb_store,
	url: *const c_char,
	out_header: *mut bkb_collection_header,
) -> bkb_error_code {
	run_ffi(|| {
		if out_header.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?.to_owned();
		let value = unsafe {
			with_store_async(store, |store| async move {
				store.collection_fetch_header(&url).await
			})
		}?;
		unsafe { out_header.write(collection_header(value)) };
		Ok(())
	})
}

/// Fetch and insert this collection to your backbeat store.
///
/// Updates it if already installed.
///
/// To install the contents inside this collection, use bkb_store_collection_fetch_download_data.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_collection_fetch_upsert(
	store: *const bkb_store,
	url: *const c_char,
	out_result: *mut bkb_collection_upsert_result,
) -> bkb_error_code {
	run_ffi(|| {
		if out_result.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?.to_owned();
		let value = unsafe {
			with_store_async(store, |store| async move {
				store.collection_fetch_upsert(&url).await
			})
		}?;
		unsafe { out_result.write(collection_upsert_result(value)) };
		Ok(())
	})
}

/// Release a collection download data report returned by bkb_store_collection_fetch_download_data.
///
/// # Safety
///
/// The value must have been returned by the corresponding Backbeat C SDK function and must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_collection_download_data_report_free(
	report: bkb_collection_download_data_report,
) {
	if report.errors.is_null() {
		return;
	}
	let errors = ptr::slice_from_raw_parts_mut(report.errors, report.errors_len);
	for error in unsafe { &mut *errors } {
		unsafe {
			bkb_string_free(error.item.value);
			bkb_string_free(error.message);
		}
	}
	unsafe { drop(Box::from_raw(errors)) };
}

unsafe fn download_snapshots_free(items: *const bkb_download_snapshot, items_len: usize) {
	if items.is_null() {
		return;
	}
	let items = ptr::slice_from_raw_parts_mut(items.cast_mut(), items_len);
	for item in unsafe { &mut *items } {
		unsafe {
			bkb_string_free(item.key.value);
			bkb_string_free(item.error);
		}
	}
	unsafe { drop(Box::from_raw(items)) };
}

/// Release a download snapshot list returned by bkb_store_download_all_progress.
///
/// # Safety
///
/// The value must have been returned by the corresponding Backbeat C SDK function and must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_download_snapshot_list_free(value: bkb_download_snapshot_list) {
	unsafe { download_snapshots_free(value.items, value.items_len) };
}

/// Release a download overview returned by bkb_store_download_overview.
///
/// # Safety
///
/// The value must have been returned by the corresponding Backbeat C SDK function and must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_download_overview_free(value: bkb_download_overview) {
	unsafe { bkb_string_free(value.first_error) };
}

/// Release a paginated download list returned by bkb_store_download_list or bkb_store_collection_download_list.
///
/// # Safety
///
/// The value must have been returned by the corresponding Backbeat C SDK function and must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_download_list_result_free(value: bkb_download_list_result) {
	unsafe {
		bkb_string_free(value.first_error);
		download_snapshots_free(value.downloads, value.downloads_len);
	}
}

/// Collect statistics about the store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_stats(
	store: *const bkb_store,
	out_stats: *mut bkb_stats,
) -> bkb_error_code {
	run_ffi(|| {
		if out_stats.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let StoreStats {
			charts,
			tables,
			courses,
			packs,
			asset_count,
			asset_bytes,
			db_bytes,
		} = unsafe { with_store(store, backbeat_sdk::Backbeat::stats) }?;
		unsafe {
			out_stats.write(bkb_stats {
				charts,
				tables,
				courses,
				packs,
				asset_count,
				asset_bytes,
				db_bytes,
			});
		};
		Ok(())
	})
}

/// Check for store corruption.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_corruption_check(
	store: *const bkb_store,
	out_report: *mut *mut bkb_corruption_report,
) -> bkb_error_code {
	run_ffi(|| {
		if out_report.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| store.corruption_check()) }?;
		unsafe { out_report.write(bkb_corruption_report::alloc(value)) };
		Ok(())
	})
}

/// Release a corruption report returned by bkb_store_corruption_check.
///
/// # Safety
///
/// The value must be null or a pointer returned by the corresponding Backbeat C SDK function, and it must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_corruption_report_free(value: *mut bkb_corruption_report) {
	unsafe { bkb_corruption_report::free(value) };
}

/// Repair some store corruption.
///
/// - For all charts, re-inspect it, and save the new inspected metadata.
/// - Delete corrupt or missing assets so they can be downloaded again.
/// - Delete charts whose contents are corrupt or cannot be inspected.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_corruption_repair(store: *const bkb_store) -> bkb_error_code {
	run_ffi(|| {
		unsafe { with_store(store, |store| store.corruption_repair()) }?;
		Ok(())
	})
}

/// Delete all assets that are not referenced by a chart or collection.
///
/// Returns the amount of things that were removed, or will be removed if dry_run is true.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_asset_prune(
	store: *const bkb_store,
	dry_run: bool,
	out_count: *mut u64,
) -> bkb_error_code {
	run_ffi(|| {
		if out_count.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| store.asset_prune(dry_run)) }?;
		unsafe { out_count.write(value) };
		Ok(())
	})
}

/// Delete any assets that are stored on disk, but do not exist in the store.
/// This also wipes files in .downloading that have not been touched in the past hour.
/// Under normal circumstances, this should never happen.
///
/// Returns the amount of things that were removed, or will be removed if dry_run is true.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_disk_prune(
	store: *const bkb_store,
	dry_run: bool,
	out_count: *mut u64,
) -> bkb_error_code {
	run_ffi(|| {
		if out_count.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| store.disk_prune(dry_run)) }?;
		unsafe { out_count.write(value) };
		Ok(())
	})
}

/// List installed tables, optionally restricted to gamemodes.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// If gamemodes_len is greater than zero, gamemodes must point to an array of that many non-null, NUL-terminated, valid UTF-8 C strings. If gamemodes_len is zero, gamemodes may be null.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_list_tables(
	store: *const bkb_store,
	gamemodes: *const *const c_char,
	gamemodes_len: usize,
	out_list: *mut *mut bkb_collection_metadata_list,
) -> bkb_error_code {
	run_ffi(|| {
		if out_list.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let gamemodes = unsafe { parse_gamemodes(gamemodes, gamemodes_len) }?;
		let values = unsafe { with_store(store, |store| store.list_tables(&gamemodes)) }?;
		unsafe { out_list.write(bkb_collection_metadata_list::alloc(values)) };
		Ok(())
	})
}

/// List installed courses, optionally restricted to gamemodes.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// If gamemodes_len is greater than zero, gamemodes must point to an array of that many non-null, NUL-terminated, valid UTF-8 C strings. If gamemodes_len is zero, gamemodes may be null.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_list_courses(
	store: *const bkb_store,
	gamemodes: *const *const c_char,
	gamemodes_len: usize,
	out_list: *mut *mut bkb_collection_metadata_list,
) -> bkb_error_code {
	run_ffi(|| {
		if out_list.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let gamemodes = unsafe { parse_gamemodes(gamemodes, gamemodes_len) }?;
		let values = unsafe { with_store(store, |store| store.list_courses(&gamemodes)) }?;
		unsafe { out_list.write(bkb_collection_metadata_list::alloc(values)) };
		Ok(())
	})
}

/// List installed packs, optionally restricted to gamemodes.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// If gamemodes_len is greater than zero, gamemodes must point to an array of that many non-null, NUL-terminated, valid UTF-8 C strings. If gamemodes_len is zero, gamemodes may be null.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_list_packs(
	store: *const bkb_store,
	gamemodes: *const *const c_char,
	gamemodes_len: usize,
	out_list: *mut *mut bkb_collection_metadata_list,
) -> bkb_error_code {
	run_ffi(|| {
		if out_list.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let gamemodes = unsafe { parse_gamemodes(gamemodes, gamemodes_len) }?;
		let values = unsafe { with_store(store, |store| store.list_packs(&gamemodes)) }?;
		unsafe { out_list.write(bkb_collection_metadata_list::alloc(values)) };
		Ok(())
	})
}

/// Search bundles by query, optionally restricted to file extensions. An empty extension list does not filter results.
///
/// Results are listed alphabetically if query is empty, that is, null or blank.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// query may be null to search without a query; otherwise it must be NUL-terminated and valid UTF-8. If extensions_len is greater than zero, extensions must point to an array of that many non-null, NUL-terminated, valid UTF-8 C strings. If extensions_len is zero, extensions may be null.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_search_bundles(
	store: *const bkb_store,
	query: *const c_char,
	offset: u64,
	limit: u32,
	extensions: *const *const c_char,
	extensions_len: usize,
	out_result: *mut *mut bkb_bundle_search_result,
) -> bkb_error_code {
	run_ffi(|| {
		if out_result.is_null() || (extensions_len > 0 && extensions.is_null()) {
			return Err(BKB_ERR_NULL_ARG);
		}
		let query = if query.is_null() {
			None
		} else {
			Some(unsafe { cstr_to_str(query) }?)
		};
		let extensions = if extensions_len == 0 {
			Vec::new()
		} else {
			unsafe { std::slice::from_raw_parts(extensions, extensions_len) }
				.iter()
				.map(|value| unsafe { cstr_to_str(*value) })
				.collect::<Result<Vec<_>, _>>()?
		};
		let value = unsafe {
			with_store(store, |store| {
				store.search_bundles(query, offset, limit, &extensions)
			})
		}?;
		unsafe { out_result.write(bkb_bundle_search_result::alloc(value)) };
		Ok(())
	})
}

/// Release a bundle search result returned by bkb_store_search_bundles.
///
/// # Safety
///
/// The value must be null or a pointer returned by the corresponding Backbeat C SDK function, and it must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bundle_search_result_free(value: *mut bkb_bundle_search_result) {
	unsafe { bkb_bundle_search_result::free(value) };
}

/// Release a collection metadata list returned by bkb_store_list_tables, bkb_store_list_courses, or bkb_store_list_packs.
///
/// # Safety
///
/// The value must be null or a pointer returned by the corresponding Backbeat C SDK function, and it must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_collection_metadata_list_free(
	value: *mut bkb_collection_metadata_list,
) {
	unsafe { bkb_collection_metadata_list::free(value) };
}

/// Get this pack from your database by URL.
///
/// This function does not do a network call; collections are just identified by URL. This fetches information from your store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_pack(
	store: *const bkb_store,
	url: *const c_char,
	out_pack: *mut *mut bkb_pack,
) -> bkb_error_code {
	run_ffi(|| {
		if out_pack.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?;
		let inner = unsafe { with_store(store, |store| store.get_pack(url)) }?;
		unsafe { out_pack.write(bkb_pack::alloc(inner)) };
		Ok(())
	})
}

/// Get this table from your database by URL.
///
/// This function does not do a network call. This fetches information from your store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_table(
	store: *const bkb_store,
	url: *const c_char,
	out_table: *mut *mut bkb_table,
) -> bkb_error_code {
	run_ffi(|| {
		if out_table.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?;
		let inner = unsafe { with_store(store, |store| store.get_table(url)) }?;
		unsafe { out_table.write(bkb_table::alloc(inner)) };
		Ok(())
	})
}

/// Get this course from your database by URL.
///
/// This function does not do a network call. This fetches information from your store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_course(
	store: *const bkb_store,
	url: *const c_char,
	out_course: *mut *mut bkb_course,
) -> bkb_error_code {
	run_ffi(|| {
		if out_course.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?;
		let inner = unsafe { with_store(store, |store| store.get_course(url)) }?;
		unsafe { out_course.write(bkb_course::alloc(inner)) };
		Ok(())
	})
}

/// Given a chart ID, get the chart file bytes.
///
/// To get the backbeat file for a chart, use bkb_store_get_chart.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_chart_data(
	store: *const bkb_store,
	chart_id: *const c_char,
	out_data: *mut bkb_bytes,
) -> bkb_error_code {
	run_ffi(|| {
		if out_data.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		let data = unsafe { with_store(store, |store| store.get_chart_data(&chart_id)) }?;
		unsafe { bkb_bytes::write(out_data, data) }
	})
}

/// Get the backbeat file for this chart ID. If this request is ambiguous, there are two bundles with the same chart_sha256, an unspecified bundle with the correct chart ID will be returned.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_chart(
	store: *const bkb_store,
	chart_id: *const c_char,
	out_bb: *mut *mut bkb_bb,
) -> bkb_error_code {
	run_ffi(|| {
		if out_bb.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		let inner = unsafe { with_store(store, |store| store.get_chart(&chart_id)) }?;
		unsafe { out_bb.write(bkb_bb::alloc(inner)) };
		Ok(())
	})
}

/// Get a bundle by its bundle ID.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_bundle(
	store: *const bkb_store,
	bundle_id: *const c_char,
	out_bb: *mut *mut bkb_bb,
) -> bkb_error_code {
	run_ffi(|| {
		if out_bb.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		let inner = unsafe { with_store(store, |store| store.get_bundle(bundle_id)) }?;
		unsafe { out_bb.write(bkb_bb::alloc(inner)) };
		Ok(())
	})
}

impl bkb_asset_data {
	fn new(data: AssetData) -> Result<Self, bkb_error_code> {
		Ok(match data {
			AssetData::Bytes(bytes) => Self {
				kind: BKB_ASSET_DATA_BYTES,
				value: bkb_asset_data_value {
					bytes: bkb_bytes::new(bytes),
				},
			},
			AssetData::File(path) => Self {
				kind: BKB_ASSET_DATA_FILE,
				value: bkb_asset_data_value {
					file: bkb_string::new(path_string(path)?)?,
				},
			},
		})
	}
}

/// Given an asset ID, get the actual data for this asset.
///
/// This returns bkb_asset_data, which either contains the entire file already in a buffer if small or a reader to get the file bytes off disk. See the bkb_asset_data type for more functionality and how to work with the bytes.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_get_asset(
	store: *const bkb_store,
	asset_id: *const c_char,
	out_data: *mut bkb_asset_data,
) -> bkb_error_code {
	run_ffi(|| {
		if out_data.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let asset_id = unsafe { parse_asset_id(asset_id) }?;
		let data = unsafe { with_store(store, |store| store.get_asset(asset_id)) }?;
		unsafe { out_data.write(bkb_asset_data::new(data)?) };
		Ok(())
	})
}

/// Resolve asset data for a bundle plus path.
///
/// This is like resolving a path in a backbeat file, but easy.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// bundle_id and asset_path must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_resolve_path(
	store: *const bkb_store,
	bundle_id: *const c_char,
	asset_path: *const c_char,
	out_data: *mut bkb_asset_data,
) -> bkb_error_code {
	run_ffi(|| {
		if out_data.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		let asset_path = unsafe { cstr_to_str(asset_path) }?;
		let data =
			unsafe { with_store(store, |store| store.resolve_asset(bundle_id, asset_path)) }?;
		unsafe { out_data.write(bkb_asset_data::new(data)?) };
		Ok(())
	})
}

/// Release asset data returned by bkb_store_get_asset or bkb_store_resolve_path.
///
/// # Safety
///
/// The value must have been returned by the corresponding Backbeat C SDK function and must not have already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_asset_data_free(data: bkb_asset_data) {
	match data.kind {
		BKB_ASSET_DATA_BYTES => unsafe { crate::functions::bkb_bytes_free(data.value.bytes) },
		BKB_ASSET_DATA_FILE => unsafe { crate::functions::bkb_string_free(data.value.file) },
		_ => {}
	}
}

/// Return true if this asset is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_asset(
	store: *const bkb_store,
	asset_id: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let asset_id = unsafe { parse_asset_id(asset_id) }?;
		let value = unsafe { with_store(store, |store| store.has_asset(asset_id)) }?;
		unsafe { out.write(value) };
		Ok(())
	})
}

/// Return true if a chart with this ID is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_chart(
	store: *const bkb_store,
	chart_id: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		let value = unsafe { with_store(store, |store| store.has_chart(&chart_id)) }?;
		unsafe { out.write(value) };
		Ok(())
	})
}

/// Return true if this bundle is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_bundle(
	store: *const bkb_store,
	bundle_id: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		let value = unsafe { with_store(store, |store| store.has_bundle(bundle_id)) }?;
		unsafe { out.write(value) };
		Ok(())
	})
}

/// Return true if a collection with this URL is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_collection(
	store: *const bkb_store,
	url: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	unsafe { has_collection_kind(store, url, out, |store, url| store.has_collection(url)) }
}

/// Return true if a pack with this URL is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_pack(
	store: *const bkb_store,
	url: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	unsafe { has_collection_kind(store, url, out, |store, url| store.has_pack(url)) }
}

/// Return true if a table with this URL is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_table(
	store: *const bkb_store,
	url: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	unsafe { has_collection_kind(store, url, out, |store, url| store.has_table(url)) }
}

/// Return true if a course with this URL is installed.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_has_course(
	store: *const bkb_store,
	url: *const c_char,
	out: *mut bool,
) -> bkb_error_code {
	unsafe { has_collection_kind(store, url, out, |store, url| store.has_course(url)) }
}

/// Get the progress of a download, if any.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// data_id must be non-null and point to a valid bkb_data_id for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_progress(
	store: *const bkb_store,
	data_id: *const bkb_data_id,
	out_progress: *mut bkb_optional_download_progress,
) -> bkb_error_code {
	run_ffi(|| {
		if out_progress.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let data_id = unsafe { parse_data(data_id) }?;
		let value = unsafe { with_store(store, |store| Ok(store.download_progress(&data_id))) }?;
		let value = optional_download_progress(value);
		unsafe { out_progress.write(value) };
		Ok(())
	})
}

/// What is the current state of all downloads?
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_all_progress(
	store: *const bkb_store,
	out_progress: *mut bkb_download_snapshot_list,
) -> bkb_error_code {
	run_ffi(|| {
		if out_progress.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.download_all_progress())) }?;
		let value = download_snapshot_list(value)?;
		unsafe { out_progress.write(value) };
		Ok(())
	})
}

/// Get an overview of current download stats.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_overview(
	store: *const bkb_store,
	out_overview: *mut bkb_download_overview,
) -> bkb_error_code {
	run_ffi(|| {
		if out_overview.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.download_overview())) }?;
		let value = download_overview(value)?;
		unsafe { out_overview.write(value) };
		Ok(())
	})
}

/// Paginated download list for the GUI.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_list(
	store: *const bkb_store,
	offset: u64,
	limit: u32,
	out_result: *mut bkb_download_list_result,
) -> bkb_error_code {
	run_ffi(|| {
		if out_result.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.download_list(offset, limit))) }?;
		let value = download_list_result(value)?;
		unsafe { out_result.write(value) };
		Ok(())
	})
}

/// Paginated chart and bundle downloads for the Downloads view.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_collection_download_list(
	store: *const bkb_store,
	offset: u64,
	limit: u32,
	out_result: *mut bkb_download_list_result,
) -> bkb_error_code {
	run_ffi(|| {
		if out_result.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe {
			with_store(store, |store| {
				Ok(store.collection_download_list(offset, limit))
			})
		}?;
		let value = download_list_result(value)?;
		unsafe { out_result.write(value) };
		Ok(())
	})
}

/// Remove every terminal download from the download manager.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_clear_finished(
	store: *const bkb_store,
	out_count: *mut usize,
) -> bkb_error_code {
	run_ffi(|| {
		if out_count.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = unsafe { with_store(store, |store| Ok(store.download_clear_finished())) }?;
		unsafe { out_count.write(value) };
		Ok(())
	})
}

/// Cancel an in-flight download.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// data_id must be non-null and point to a valid bkb_data_id for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_cancel(
	store: *const bkb_store,
	data_id: *const bkb_data_id,
	out_cancelled: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out_cancelled.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let data_id = unsafe { parse_data(data_id) }?;
		let value = unsafe { with_store(store, |store| Ok(store.download_cancel(data_id))) }?;
		unsafe { out_cancelled.write(value) };
		Ok(())
	})
}

/// Cancel an in-flight asset download.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_asset_cancel(
	store: *const bkb_store,
	asset_id: *const c_char,
	out_cancelled: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out_cancelled.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let asset_id = unsafe { parse_asset_id(asset_id) }?;
		let value =
			unsafe { with_store(store, |store| Ok(store.download_asset_cancel(asset_id))) }?;
		unsafe { out_cancelled.write(value) };
		Ok(())
	})
}

/// Cancel an in-flight chart download.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_chart_cancel(
	store: *const bkb_store,
	chart_id: *const c_char,
	out_cancelled: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out_cancelled.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let chart_id = unsafe { parse_chart_id(chart_id) }?;
		let value =
			unsafe { with_store(store, |store| Ok(store.download_chart_cancel(&chart_id))) }?;
		unsafe { out_cancelled.write(value) };
		Ok(())
	})
}

/// Cancel an in-flight bundle download.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_download_bundle_cancel(
	store: *const bkb_store,
	bundle_id: *const c_char,
	out_cancelled: *mut bool,
) -> bkb_error_code {
	run_ffi(|| {
		if out_cancelled.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		let value =
			unsafe { with_store(store, |store| Ok(store.download_bundle_cancel(bundle_id))) }?;
		unsafe { out_cancelled.write(value) };
		Ok(())
	})
}

/// Remove this bundle from your store.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_bundle_rm(
	store: *const bkb_store,
	bundle_id: *const c_char,
) -> bkb_error_code {
	run_ffi(|| {
		let bundle_id = unsafe { parse_bundle_id(bundle_id) }?;
		unsafe { with_store(store, |store| store.bundle_rm(bundle_id)) }
	})
}

/// Remove a collection from your store. Optionally, decide whether you want to remove its charts, too.
///
/// Charts will only be removed if there is not another collection referencing the chart.
///
/// # Safety
///
/// The store pointer must be non-null and point to a live bkb_store returned by bkb_store_open. It must remain valid for the duration of the call.
/// Every C string pointer must be non-null, NUL-terminated, and point to valid UTF-8 for the duration of the call.
/// Every output pointer must be non-null and point to writable storage of the type shown. If this function returns an error, output storage may be unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_store_collection_rm(
	store: *const bkb_store,
	url: *const c_char,
	remove_charts_too: bool,
	out_kind: *mut bkb_collection_kind,
) -> bkb_error_code {
	run_ffi(|| {
		if out_kind.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?;
		let value =
			unsafe { with_store(store, |store| store.collection_rm(url, remove_charts_too)) }?;
		unsafe { out_kind.write(collection_kind(value)) };
		Ok(())
	})
}

#[cfg(test)]
#[path = "store/method_tests.rs"]
mod method_tests;

#[cfg(test)]
#[path = "store/tests.rs"]
mod tests;
