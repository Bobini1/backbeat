use std::borrow::Borrow;
use std::ffi::{CString, OsString};
use std::future::Future;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::sync::OnceLock;

use backbeat_core::{AssetId, BundleId, ChartId, CollectionKind};
use backbeat_sdk::collections::{CollectionDownloadDataFailure, CollectionDownloadDataReport};
use backbeat_sdk::download_manager::{
	DownloadListResult, DownloadOverview, DownloadProgress, DownloadSnapshot, DownloadState,
};
use backbeat_sdk::{Backbeat, DataId, StoreError};
use backbeat_store_config::ServerConfig;
use futures::FutureExt;

use super::{
	BKB_COLLECTION_COURSE, BKB_COLLECTION_PACK, BKB_COLLECTION_TABLE, BKB_DATA_ASSET,
	BKB_DATA_BUNDLE, BKB_DATA_CHART, BKB_DOWNLOAD_CANCELLED, BKB_DOWNLOAD_COMMITTING,
	BKB_DOWNLOAD_DONE, BKB_DOWNLOAD_DOWNLOADING, BKB_DOWNLOAD_FAILED, BKB_DOWNLOAD_QUEUED,
	BKB_DOWNLOAD_VERIFYING, bkb_collection_download_data_failure,
	bkb_collection_download_data_report, bkb_collection_kind, bkb_data_id, bkb_data_kind,
	bkb_download_list_result, bkb_download_overview, bkb_download_progress, bkb_download_snapshot,
	bkb_download_snapshot_list, bkb_download_state, bkb_optional_download_progress,
	bkb_optional_u64, bkb_owned_data_id,
};
use crate::error::{
	BKB_ERR_INVALID_STRING, BKB_ERR_INVALID_URL, BKB_ERR_NULL_ARG, BKB_ERR_PARSE, bkb_error_code,
	store_error_code,
};
use crate::types::{bkb_server_config, bkb_store, cstr_to_str};
use crate::util::run_ffi;

static EXECUTOR: OnceLock<futures::executor::ThreadPool> = OnceLock::new();

fn executor() -> &'static futures::executor::ThreadPool {
	EXECUTOR.get_or_init(|| {
		futures::executor::ThreadPoolBuilder::new()
			.pool_size(1)
			.name_prefix("backbeat-c-sdk-")
			.create()
			.expect("failed to build C SDK executor")
	})
}

pub(super) fn run_store<T>(
	op: impl FnOnce() -> backbeat_sdk::Result<T>,
) -> Result<T, bkb_error_code> {
	op().map_err(|err| store_error_code(&err))
}

pub(super) unsafe fn with_store<T>(
	store: *const bkb_store,
	op: impl FnOnce(&Backbeat) -> backbeat_sdk::Result<T>,
) -> Result<T, bkb_error_code> {
	if store.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	let store = unsafe { &(*store).inner };
	run_store(|| op(store))
}

pub(super) unsafe fn with_store_async<T, E, F>(
	store: *const bkb_store,
	op: impl FnOnce(Backbeat) -> F,
) -> Result<T, bkb_error_code>
where
	E: Borrow<StoreError>,
	F: Future<Output = Result<T, E>>,
{
	if store.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	let store = unsafe { (*store).inner.clone() };
	let result = futures_lite::future::block_on(op(store));
	result.map_err(|error| store_error_code(error.borrow()))
}

pub(super) unsafe fn with_store_spawn<T, F>(
	store: *const bkb_store,
	op: impl FnOnce(Backbeat) -> F,
) -> Result<(), bkb_error_code>
where
	T: Send + 'static,
	F: Future<Output = T> + Send + 'static,
{
	if store.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	let store = unsafe { (*store).inner.clone() };
	let future = op(store);
	executor().spawn_ok(async move {
		let _ = AssertUnwindSafe(future).catch_unwind().await;
	});
	Ok(())
}

pub(super) fn path_string(path: impl Into<OsString>) -> Result<String, bkb_error_code> {
	path.into()
		.into_string()
		.map_err(|_| BKB_ERR_INVALID_STRING)
}

fn optional_u64(value: Option<u64>) -> bkb_optional_u64 {
	match value {
		Some(value) => bkb_optional_u64 {
			is_some: true,
			value,
		},
		None => bkb_optional_u64 {
			is_some: false,
			value: 0,
		},
	}
}

fn download_state(value: DownloadState) -> bkb_download_state {
	match value {
		DownloadState::Queued => BKB_DOWNLOAD_QUEUED,
		DownloadState::Downloading => BKB_DOWNLOAD_DOWNLOADING,
		DownloadState::Verifying => BKB_DOWNLOAD_VERIFYING,
		DownloadState::Committing => BKB_DOWNLOAD_COMMITTING,
		DownloadState::Done => BKB_DOWNLOAD_DONE,
		DownloadState::Failed => BKB_DOWNLOAD_FAILED,
		DownloadState::Cancelled => BKB_DOWNLOAD_CANCELLED,
	}
}

fn download_progress(value: DownloadProgress) -> bkb_download_progress {
	bkb_download_progress {
		bytes: value.bytes,
		total: optional_u64(value.total),
		items_done: value.items_done,
		items_total: optional_u64(value.items_total),
		state: download_state(value.state),
	}
}

struct PreparedDownloadSnapshot {
	kind: bkb_data_kind,
	value: CString,
	progress: DownloadProgress,
	error: Option<CString>,
}

impl PreparedDownloadSnapshot {
	fn new(value: DownloadSnapshot) -> Result<Self, bkb_error_code> {
		let DownloadSnapshot {
			key,
			progress,
			error,
		} = value;
		let (kind, value) = data_id_parts(key);
		Ok(Self {
			kind,
			value: CString::new(value).map_err(|_| BKB_ERR_INVALID_STRING)?,
			progress,
			error: error
				.map(CString::new)
				.transpose()
				.map_err(|_| BKB_ERR_INVALID_STRING)?,
		})
	}

	fn into_ffi(self) -> bkb_download_snapshot {
		bkb_download_snapshot {
			key: bkb_owned_data_id {
				kind: self.kind,
				value: owned_c_string(self.value),
			},
			progress: download_progress(self.progress),
			error: optional_owned_c_string(self.error),
		}
	}
}

fn data_id_parts(value: DataId) -> (bkb_data_kind, String) {
	match value {
		DataId::Chart(id) => (BKB_DATA_CHART, id.to_string()),
		DataId::Bundle(id) => (BKB_DATA_BUNDLE, id.to_string()),
		DataId::Asset(id) => (BKB_DATA_ASSET, id.to_string()),
	}
}

fn download_snapshots(
	values: Vec<DownloadSnapshot>,
) -> Result<(*const bkb_download_snapshot, usize), bkb_error_code> {
	let values = values
		.into_iter()
		.map(PreparedDownloadSnapshot::new)
		.collect::<Result<Vec<_>, _>>()?
		.into_iter()
		.map(PreparedDownloadSnapshot::into_ffi)
		.collect::<Vec<_>>();
	let len = values.len();
	let values = if values.is_empty() {
		std::ptr::null_mut()
	} else {
		Box::into_raw(values.into_boxed_slice()).cast()
	};
	Ok((values, len))
}

pub(super) fn download_snapshot_list(
	values: Vec<DownloadSnapshot>,
) -> Result<bkb_download_snapshot_list, bkb_error_code> {
	let (items, items_len) = download_snapshots(values)?;
	Ok(bkb_download_snapshot_list { items, items_len })
}

pub(super) fn download_overview(
	value: DownloadOverview,
) -> Result<bkb_download_overview, bkb_error_code> {
	let first_error = value
		.first_error
		.map(CString::new)
		.transpose()
		.map_err(|_| BKB_ERR_INVALID_STRING)?;
	Ok(bkb_download_overview {
		total: value.total,
		queued: value.queued,
		running: value.running,
		failed: value.failed,
		done: value.done,
		cancelled: value.cancelled,
		first_error: optional_owned_c_string(first_error),
	})
}

pub(super) fn download_list_result(
	value: DownloadListResult,
) -> Result<bkb_download_list_result, bkb_error_code> {
	let first_error = value
		.first_error
		.map(CString::new)
		.transpose()
		.map_err(|_| BKB_ERR_INVALID_STRING)?;
	let (downloads, downloads_len) = download_snapshots(value.downloads)?;
	Ok(bkb_download_list_result {
		total: value.total,
		queued: value.queued,
		running: value.running,
		failed: value.failed,
		done: value.done,
		cancelled: value.cancelled,
		first_error: optional_owned_c_string(first_error),
		downloads,
		downloads_len,
		has_more: value.has_more,
	})
}

pub(super) fn optional_download_progress(
	value: Option<DownloadProgress>,
) -> bkb_optional_download_progress {
	match value {
		Some(value) => bkb_optional_download_progress {
			is_some: true,
			value: download_progress(value),
		},
		None => bkb_optional_download_progress {
			is_some: false,
			value: bkb_download_progress {
				bytes: 0,
				total: optional_u64(None),
				items_done: 0,
				items_total: optional_u64(None),
				state: BKB_DOWNLOAD_QUEUED,
			},
		},
	}
}

pub(super) unsafe fn server_config(
	value: *const bkb_server_config,
) -> Result<ServerConfig, bkb_error_code> {
	if value.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	let url = unsafe { cstr_to_str((*value).url) }?;
	url::Url::parse(url).map_err(|_| BKB_ERR_INVALID_URL)?;
	Ok(ServerConfig { url: url.into() })
}

pub(super) unsafe fn parse_asset_id(value: *const c_char) -> Result<AssetId, bkb_error_code> {
	unsafe { cstr_to_str(value) }?
		.parse()
		.map_err(|_| BKB_ERR_PARSE)
}

pub(super) unsafe fn parse_bundle_id(value: *const c_char) -> Result<BundleId, bkb_error_code> {
	unsafe { cstr_to_str(value) }?
		.parse()
		.map_err(|_| BKB_ERR_PARSE)
}

pub(super) unsafe fn parse_chart_id(value: *const c_char) -> Result<ChartId, bkb_error_code> {
	unsafe { cstr_to_str(value) }?
		.parse()
		.map_err(|_| BKB_ERR_PARSE)
}

pub(super) unsafe fn parse_data(value: *const bkb_data_id) -> Result<DataId, bkb_error_code> {
	if value.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}

	unsafe {
		match (*value).kind {
			BKB_DATA_CHART => Ok(DataId::Chart(parse_chart_id((*value).value)?)),
			BKB_DATA_BUNDLE => Ok(DataId::Bundle(parse_bundle_id((*value).value)?)),
			BKB_DATA_ASSET => Ok(DataId::Asset(parse_asset_id((*value).value)?)),
			_ => Err(BKB_ERR_PARSE),
		}
	}
}

pub(super) fn collection_kind(value: CollectionKind) -> bkb_collection_kind {
	match value {
		CollectionKind::Table => BKB_COLLECTION_TABLE,
		CollectionKind::Course => BKB_COLLECTION_COURSE,
		CollectionKind::Pack => BKB_COLLECTION_PACK,
	}
}

fn owned_c_string(value: CString) -> crate::types::bkb_string {
	let len = value.as_bytes().len();
	crate::types::bkb_string {
		ptr: value.into_raw(),
		len,
	}
}

fn optional_owned_c_string(value: Option<CString>) -> crate::types::bkb_string {
	value.map_or(
		crate::types::bkb_string {
			ptr: std::ptr::null_mut(),
			len: 0,
		},
		owned_c_string,
	)
}

pub(super) fn collection_download_data_report(
	value: CollectionDownloadDataReport,
) -> Result<bkb_collection_download_data_report, bkb_error_code> {
	let CollectionDownloadDataReport {
		charts_downloaded,
		charts_skipped,
		charts_failed,
		bundles_downloaded,
		bundles_skipped,
		bundles_failed,
		assets_downloaded,
		assets_skipped,
		assets_failed,
		errors,
	} = value;
	let errors = errors
		.into_iter()
		.map(|CollectionDownloadDataFailure { item, message }| {
			let (kind, item) = match item {
				DataId::Chart(id) => (BKB_DATA_CHART, id.to_string()),
				DataId::Bundle(id) => (BKB_DATA_BUNDLE, id.to_string()),
				DataId::Asset(id) => (BKB_DATA_ASSET, id.to_string()),
			};
			let item = CString::new(item).map_err(|_| BKB_ERR_INVALID_STRING)?;
			let message = CString::new(message).map_err(|_| BKB_ERR_INVALID_STRING)?;
			Ok((kind, item, message))
		})
		.collect::<Result<Vec<_>, bkb_error_code>>()?
		.into_iter()
		.map(
			|(kind, item, message)| bkb_collection_download_data_failure {
				item: bkb_owned_data_id {
					kind,
					value: owned_c_string(item),
				},
				message: owned_c_string(message),
			},
		)
		.collect::<Vec<_>>();
	let errors_len = errors.len();
	let errors = Box::into_raw(errors.into_boxed_slice()).cast();
	Ok(bkb_collection_download_data_report {
		charts_downloaded,
		charts_skipped,
		charts_failed,
		bundles_downloaded,
		bundles_skipped,
		bundles_failed,
		assets_downloaded,
		assets_skipped,
		assets_failed,
		errors,
		errors_len,
	})
}

pub(super) unsafe fn has_collection_kind(
	store: *const bkb_store,
	url: *const c_char,
	out: *mut bool,
	op: impl FnOnce(&Backbeat, &str) -> backbeat_sdk::Result<bool>,
) -> bkb_error_code {
	run_ffi(|| {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let url = unsafe { cstr_to_str(url) }?;
		let value = unsafe { with_store(store, |store| op(store, url)) }?;
		unsafe { out.write(value) };
		Ok(())
	})
}
