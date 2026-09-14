use std::os::raw::c_char;

use crate::types::{bkb_bytes, bkb_str, bkb_string, bkb_timestamp};

pub type bkb_asset_data_kind = i32;

pub const BKB_ASSET_DATA_BYTES: bkb_asset_data_kind = 1;
pub const BKB_ASSET_DATA_FILE: bkb_asset_data_kind = 2;

#[derive(Clone, Copy)]
#[repr(C)]
pub union bkb_asset_data_value {
	pub bytes: bkb_bytes,
	pub file: bkb_string,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_asset_data {
	pub kind: bkb_asset_data_kind,
	pub value: bkb_asset_data_value,
}

pub type bkb_data_kind = i32;

pub const BKB_DATA_CHART: bkb_data_kind = 1;
pub const BKB_DATA_BUNDLE: bkb_data_kind = 2;
pub const BKB_DATA_ASSET: bkb_data_kind = 3;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_data_id {
	pub kind: bkb_data_kind,
	pub value: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_owned_data_id {
	pub kind: bkb_data_kind,
	pub value: bkb_string,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_collection_download_data_failure {
	pub item: bkb_owned_data_id,
	pub message: bkb_string,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_collection_download_data_report {
	pub charts_downloaded: u64,
	pub charts_skipped: u64,
	pub charts_failed: u64,
	pub bundles_downloaded: u64,
	pub bundles_skipped: u64,
	pub bundles_failed: u64,
	pub assets_downloaded: u64,
	pub assets_skipped: u64,
	pub assets_failed: u64,
	pub errors: *mut bkb_collection_download_data_failure,
	pub errors_len: usize,
}

pub type bkb_collection_kind = i32;

pub const BKB_COLLECTION_TABLE: bkb_collection_kind = 1;
pub const BKB_COLLECTION_COURSE: bkb_collection_kind = 2;
pub const BKB_COLLECTION_PACK: bkb_collection_kind = 3;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_collection_header {
	pub last_modified: bkb_timestamp,
	pub kind: bkb_collection_kind,
}

pub type bkb_collection_upsert_status = i32;

pub const BKB_COLLECTION_UPSERT_INSERTED: bkb_collection_upsert_status = 1;
pub const BKB_COLLECTION_UPSERT_UPDATED: bkb_collection_upsert_status = 2;
pub const BKB_COLLECTION_UPSERT_TIMESTAMP_UNCHANGED: bkb_collection_upsert_status = 3;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_collection_upsert_result {
	pub kind: bkb_collection_kind,
	pub status: bkb_collection_upsert_status,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_wrong_chart_id {
	pub chart_sha256: bkb_str,
	pub alg: bkb_str,
	pub stored_id: bkb_str,
	pub computed_id: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_uncomputed_chart_id {
	pub chart_sha256: bkb_str,
	pub alg: bkb_str,
	pub reason: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_dangling_asset_ref {
	pub bundle_id: bkb_str,
	pub path: bkb_str,
	pub asset_id: bkb_str,
}

#[repr(C)]
pub struct bkb_corruption_report {
	pub large_asset_count: usize,
	pub chart_count: usize,
	pub chart_id_count: usize,
	pub missing_assets: *const bkb_str,
	pub missing_assets_len: usize,
	pub corrupt_assets: *const bkb_str,
	pub corrupt_assets_len: usize,
	pub corrupt_charts: *const bkb_str,
	pub corrupt_charts_len: usize,
	pub wrong_chart_ids: *const bkb_wrong_chart_id,
	pub wrong_chart_ids_len: usize,
	pub uncomputable_chart_ids: *const bkb_uncomputed_chart_id,
	pub uncomputable_chart_ids_len: usize,
	pub dangling_asset_refs: *const bkb_dangling_asset_ref,
	pub dangling_asset_refs_len: usize,
	pub ok: bool,
	pub issue_count: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_stats {
	pub charts: u64,
	pub tables: u64,
	pub courses: u64,
	pub packs: u64,
	pub asset_count: u64,
	pub asset_bytes: u64,
	pub db_bytes: u64,
}

pub type bkb_download_state = i32;

pub const BKB_DOWNLOAD_QUEUED: bkb_download_state = 0;
pub const BKB_DOWNLOAD_DOWNLOADING: bkb_download_state = 1;
pub const BKB_DOWNLOAD_VERIFYING: bkb_download_state = 2;
pub const BKB_DOWNLOAD_COMMITTING: bkb_download_state = 3;
pub const BKB_DOWNLOAD_DONE: bkb_download_state = 4;
pub const BKB_DOWNLOAD_FAILED: bkb_download_state = 5;
pub const BKB_DOWNLOAD_CANCELLED: bkb_download_state = 6;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_optional_u64 {
	pub is_some: bool,
	pub value: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_download_progress {
	pub bytes: u64,
	pub total: bkb_optional_u64,
	pub items_done: u64,
	pub items_total: bkb_optional_u64,
	pub state: bkb_download_state,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_optional_download_progress {
	pub is_some: bool,
	pub value: bkb_download_progress,
}

#[derive(Clone, Copy)]
#[repr(C)]
/// One download snapshot. Strings are owned by the containing result.
pub struct bkb_download_snapshot {
	pub key: bkb_owned_data_id,
	pub progress: bkb_download_progress,
	/// Null when the download has no failure message.
	pub error: bkb_string,
}

#[derive(Clone, Copy)]
#[repr(C)]
/// Library-owned download snapshots. Free with `bkb_download_snapshot_list_free`.
pub struct bkb_download_snapshot_list {
	pub items: *const bkb_download_snapshot,
	pub items_len: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
/// Library-owned download overview. Free with `bkb_download_overview_free`.
pub struct bkb_download_overview {
	pub total: u64,
	pub queued: u64,
	pub running: u64,
	pub failed: u64,
	pub done: u64,
	pub cancelled: u64,
	/// Null when no download has failed.
	pub first_error: bkb_string,
}

#[derive(Clone, Copy)]
#[repr(C)]
/// Library-owned paginated download result. Free with `bkb_download_list_result_free`.
pub struct bkb_download_list_result {
	pub total: u64,
	pub queued: u64,
	pub running: u64,
	pub failed: u64,
	pub done: u64,
	pub cancelled: u64,
	/// Null when no download has failed.
	pub first_error: bkb_string,
	pub downloads: *const bkb_download_snapshot,
	pub downloads_len: usize,
	pub has_more: bool,
}
