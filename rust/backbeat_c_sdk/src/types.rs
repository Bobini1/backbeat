use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use backbeat_core::{Assets, BackbeatFile};
use backbeat_sdk::Backbeat;
use backbeat_sdk::bundle::search::{BundleSearchResult, ChartEntry};
use backbeat_sdk::collections::{
	CollectionMetadata, CourseContents, PackContents, TableContents, TableContentsChart,
};
use backbeat_sdk::maintenance::check::CorruptionReport;

use crate::error::{BKB_ERR_INVALID_STRING, BKB_ERR_NULL_ARG, bkb_error_code};
use crate::store::{
	bkb_corruption_report, bkb_dangling_asset_ref, bkb_uncomputed_chart_id, bkb_wrong_chart_id,
};

#[derive(Clone, Copy)]
#[repr(C)]
/// Borrowed UTF-8 bytes. The data is not NUL-terminated.
pub struct bkb_str {
	pub ptr: *const c_char,
	pub len: usize,
}

#[repr(C)]
/// Library-owned `.bb` data. Treat all fields as read-only and free with `bkb_bb_free`.
pub struct bkb_bb {
	pub filename: bkb_str,
	/// Asset path-to-ID mappings contained in this bundle.
	pub assets: *const bkb_asset,
	pub assets_len: usize,
	pub desc: bkb_str,
	/// Null when `chart_len` is zero.
	pub chart: *const u8,
	pub chart_len: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_timestamp {
	pub seconds: i64,
	pub nanoseconds: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_collection_metadata {
	pub url: bkb_str,
	pub name: bkb_str,
	pub gamemode: bkb_str,
	pub updated: bkb_timestamp,
	pub installed: u64,
	pub out_of: u64,
}

#[repr(C)]
pub struct bkb_collection_metadata_list {
	pub items: *const bkb_collection_metadata,
	pub items_len: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
/// One chart returned by `bkb_store_search_bundles`.
pub struct bkb_bundle_search_chart {
	pub bundle_id: bkb_str,
	pub description: bkb_str,
	/// Null when the bundle has no file extension.
	pub extension: bkb_str,
}

#[repr(C)]
/// Library-owned page returned by `bkb_store_search_bundles`.
pub struct bkb_bundle_search_result {
	pub charts: *const bkb_bundle_search_chart,
	pub charts_len: usize,
	pub has_more: bool,
	pub total: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_tag {
	pub key: bkb_str,
	pub value: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_asset {
	pub path: bkb_str,
	pub id: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_pack_bundle {
	pub id: bkb_str,
	pub desc: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub installed: bool,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_course_chart {
	pub id: bkb_str,
	pub desc: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub bundle_id: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_table_chart {
	pub id: bkb_str,
	pub desc: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub bundle_id: bkb_str,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_table_level {
	pub level: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub charts: *const bkb_table_chart,
	pub charts_len: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_table_folder {
	pub name: bkb_str,
	pub query: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub charts: *const bkb_table_chart,
	pub charts_len: usize,
}

#[repr(C)]
/// Library-owned course data. Treat all fields as read-only and free with `bkb_course_free`.
pub struct bkb_course {
	pub name: bkb_str,
	pub updated: bkb_timestamp,
	pub gamemode: bkb_str,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub assets: *const bkb_asset,
	pub assets_len: usize,
	pub charts: *const bkb_course_chart,
	pub charts_len: usize,
}

#[repr(C)]
/// Library-owned pack data. Treat all fields as read-only and free with `bkb_pack_free`.
pub struct bkb_pack {
	pub name: bkb_str,
	pub gamemode: bkb_str,
	pub updated: bkb_timestamp,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub assets: *const bkb_asset,
	pub assets_len: usize,
	pub bundles: *const bkb_pack_bundle,
	pub bundles_len: usize,
}

pub struct bkb_store {
	pub(crate) inner: Backbeat,
}

#[repr(C)]
pub struct bkb_table {
	pub name: bkb_str,
	pub symbol: bkb_str,
	pub gamemode: bkb_str,
	pub updated: bkb_timestamp,
	pub tags: *const bkb_tag,
	pub tags_len: usize,
	pub assets: *const bkb_asset,
	pub assets_len: usize,
	pub levels: *const bkb_table_level,
	pub levels_len: usize,
	pub folders: *const bkb_table_folder,
	pub folders_len: usize,
}

#[derive(Default)]
struct CollectionViews {
	strings: Vec<Box<[u8]>>,
	tag_arrays: Vec<Box<[bkb_tag]>>,
	asset_arrays: Vec<Box<[bkb_asset]>>,
}

impl CollectionViews {
	fn string(&mut self, value: impl AsRef<str>) -> bkb_str {
		static EMPTY: [u8; 1] = [0];
		let value = value.as_ref();
		if value.is_empty() {
			return bkb_str {
				ptr: EMPTY.as_ptr().cast(),
				len: 0,
			};
		}
		let value = value.as_bytes().to_vec().into_boxed_slice();
		let result = bkb_str {
			ptr: value.as_ptr().cast(),
			len: value.len(),
		};
		self.strings.push(value);
		result
	}

	fn tags<'a>(
		&mut self,
		values: impl Iterator<Item = (&'a String, &'a String)>,
	) -> (*const bkb_tag, usize) {
		let mut values = values.collect::<Vec<_>>();
		values.sort_unstable_by(|left, right| left.0.cmp(right.0));
		let values = values
			.into_iter()
			.map(|(key, value)| bkb_tag {
				key: self.string(key),
				value: self.string(value),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let result = slice_parts(&values);
		self.tag_arrays.push(values);
		result
	}

	fn assets(&mut self, values: &Assets) -> (*const bkb_asset, usize) {
		let mut values = values
			.iter()
			.map(|(path, id)| (path.to_string(), id.to_string()))
			.collect::<Vec<_>>();
		values.sort_unstable_by(|left, right| left.0.cmp(&right.0));
		let values = values
			.into_iter()
			.map(|(path, id)| bkb_asset {
				path: self.string(path),
				id: self.string(id),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let result = slice_parts(&values);
		self.asset_arrays.push(values);
		result
	}

	fn table_chart(&mut self, chart: &TableContentsChart) -> bkb_table_chart {
		let (tags, tags_len) = self.tags(chart.tags.iter());
		let bundle_id = chart
			.bundle_id
			.map(|value| self.string(value.to_string()))
			.unwrap_or_else(null_str);
		bkb_table_chart {
			id: self.string(chart.id.to_string()),
			desc: self.string(&chart.desc),
			tags,
			tags_len,
			bundle_id,
		}
	}
}

struct CollectionMetadataListOwner {
	_views: CollectionViews,
	_items: Box<[bkb_collection_metadata]>,
}

#[repr(C)]
struct CollectionMetadataListAllocation {
	public: bkb_collection_metadata_list,
	_owner: CollectionMetadataListOwner,
}

const _: () = assert!(std::mem::offset_of!(CollectionMetadataListAllocation, public) == 0);

impl bkb_collection_metadata_list {
	pub(crate) fn alloc(values: Vec<CollectionMetadata>) -> *mut Self {
		let mut views = CollectionViews::default();
		let items = values
			.into_iter()
			.map(|value| bkb_collection_metadata {
				url: views.string(value.url),
				name: views.string(value.name),
				gamemode: views.string(value.gamemode.into_string()),
				updated: timestamp(
					value.updated.timestamp(),
					value.updated.timestamp_subsec_nanos(),
				),
				installed: value.installed,
				out_of: value.total,
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let (items_ptr, items_len) = slice_parts(&items);
		let allocation = Box::new(CollectionMetadataListAllocation {
			public: Self {
				items: items_ptr,
				items_len,
			},
			_owner: CollectionMetadataListOwner {
				_views: views,
				_items: items,
			},
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe {
				drop(Box::from_raw(
					value.cast::<CollectionMetadataListAllocation>(),
				));
			};
		}
	}
}

struct BundleSearchResultOwner {
	_views: CollectionViews,
	_charts: Box<[bkb_bundle_search_chart]>,
}

#[repr(C)]
struct BundleSearchResultAllocation {
	public: bkb_bundle_search_result,
	_owner: BundleSearchResultOwner,
}

const _: () = assert!(std::mem::offset_of!(BundleSearchResultAllocation, public) == 0);

impl bkb_bundle_search_result {
	pub(crate) fn alloc(value: BundleSearchResult) -> *mut Self {
		let mut views = CollectionViews::default();
		let charts = value
			.charts
			.into_iter()
			.map(|chart: ChartEntry| bkb_bundle_search_chart {
				bundle_id: views.string(chart.bundle_id.to_string()),
				description: views.string(chart.description),
				extension: chart
					.extension
					.map(|value| views.string(value))
					.unwrap_or_else(null_str),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let (charts_ptr, charts_len) = slice_parts(&charts);
		let allocation = Box::new(BundleSearchResultAllocation {
			public: Self {
				charts: charts_ptr,
				charts_len,
				has_more: value.has_more,
				total: value.total,
			},
			_owner: BundleSearchResultOwner {
				_views: views,
				_charts: charts,
			},
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<BundleSearchResultAllocation>())) };
		}
	}
}

struct CorruptionReportOwner {
	_views: CollectionViews,
	_missing_assets: Box<[bkb_str]>,
	_corrupt_assets: Box<[bkb_str]>,
	_corrupt_charts: Box<[bkb_str]>,
	_wrong_chart_ids: Box<[bkb_wrong_chart_id]>,
	_uncomputable_chart_ids: Box<[bkb_uncomputed_chart_id]>,
	_dangling_asset_refs: Box<[bkb_dangling_asset_ref]>,
}

#[repr(C)]
struct CorruptionReportAllocation {
	public: bkb_corruption_report,
	_owner: CorruptionReportOwner,
}

const _: () = assert!(std::mem::offset_of!(CorruptionReportAllocation, public) == 0);

impl bkb_corruption_report {
	pub(crate) fn alloc(value: CorruptionReport) -> *mut Self {
		let ok = value.is_ok();
		let issue_count = value.issue_count();
		let mut views = CollectionViews::default();
		let missing_assets = value
			.missing_assets
			.into_iter()
			.map(|value| views.string(value.to_string()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let corrupt_assets = value
			.corrupt_assets
			.into_iter()
			.map(|value| views.string(value.to_string()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let corrupt_charts = value
			.corrupt_charts
			.into_iter()
			.map(|value| views.string(value.to_string()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let wrong_chart_ids = value
			.wrong_chart_ids
			.into_iter()
			.map(|value| bkb_wrong_chart_id {
				chart_sha256: views.string(value.chart_sha256.to_string()),
				alg: views.string(value.alg),
				stored_id: views.string(value.stored_id),
				computed_id: views.string(value.computed_id),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let uncomputable_chart_ids = value
			.uncomputable_chart_ids
			.into_iter()
			.map(|value| bkb_uncomputed_chart_id {
				chart_sha256: views.string(value.chart_sha256.to_string()),
				alg: views.string(value.alg),
				reason: views.string(value.reason),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let dangling_asset_refs = value
			.dangling_asset_refs
			.into_iter()
			.map(|value| bkb_dangling_asset_ref {
				bundle_id: views.string(value.bundle_id),
				path: views.string(value.path),
				asset_id: views.string(value.asset_id.to_string()),
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let (missing_assets_ptr, missing_assets_len) = slice_parts(&missing_assets);
		let (corrupt_assets_ptr, corrupt_assets_len) = slice_parts(&corrupt_assets);
		let (corrupt_charts_ptr, corrupt_charts_len) = slice_parts(&corrupt_charts);
		let (wrong_chart_ids_ptr, wrong_chart_ids_len) = slice_parts(&wrong_chart_ids);
		let (uncomputable_chart_ids_ptr, uncomputable_chart_ids_len) =
			slice_parts(&uncomputable_chart_ids);
		let (dangling_asset_refs_ptr, dangling_asset_refs_len) = slice_parts(&dangling_asset_refs);
		let allocation = Box::new(CorruptionReportAllocation {
			public: Self {
				large_asset_count: value.large_asset_count,
				chart_count: value.chart_count,
				chart_id_count: value.chart_id_count,
				missing_assets: missing_assets_ptr,
				missing_assets_len,
				corrupt_assets: corrupt_assets_ptr,
				corrupt_assets_len,
				corrupt_charts: corrupt_charts_ptr,
				corrupt_charts_len,
				wrong_chart_ids: wrong_chart_ids_ptr,
				wrong_chart_ids_len,
				uncomputable_chart_ids: uncomputable_chart_ids_ptr,
				uncomputable_chart_ids_len,
				dangling_asset_refs: dangling_asset_refs_ptr,
				dangling_asset_refs_len,
				ok,
				issue_count,
			},
			_owner: CorruptionReportOwner {
				_views: views,
				_missing_assets: missing_assets,
				_corrupt_assets: corrupt_assets,
				_corrupt_charts: corrupt_charts,
				_wrong_chart_ids: wrong_chart_ids,
				_uncomputable_chart_ids: uncomputable_chart_ids,
				_dangling_asset_refs: dangling_asset_refs,
			},
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<CorruptionReportAllocation>())) };
		}
	}
}

fn slice_parts<T>(values: &[T]) -> (*const T, usize) {
	if values.is_empty() {
		(ptr::null(), 0)
	} else {
		(values.as_ptr(), values.len())
	}
}

fn null_str() -> bkb_str {
	bkb_str {
		ptr: ptr::null(),
		len: 0,
	}
}

fn timestamp(seconds: i64, nanoseconds: u32) -> bkb_timestamp {
	bkb_timestamp {
		seconds,
		nanoseconds,
	}
}

struct BbOwner {
	inner: BackbeatFile,
	_views: CollectionViews,
	_filename: Box<[u8]>,
	_desc: Box<[u8]>,
	_chart: Box<[u8]>,
}

#[repr(C)]
struct BbAllocation {
	public: bkb_bb,
	owner: BbOwner,
}

const _: () = assert!(std::mem::offset_of!(BbAllocation, public) == 0);

impl bkb_bb {
	pub(crate) fn alloc(inner: BackbeatFile) -> *mut Self {
		let mut views = CollectionViews::default();
		let (assets, assets_len) = views.assets(&inner.assets);
		let filename = inner
			.filename
			.as_str()
			.as_bytes()
			.to_vec()
			.into_boxed_slice();
		let desc = inner.desc.as_str().as_bytes().to_vec().into_boxed_slice();
		// i think this just makes things easier
		// and less miserable for the caller
		let chart = inner.chart.decompress().into_boxed_slice();
		let filename_view = bkb_str {
			ptr: filename.as_ptr().cast(),
			len: filename.len(),
		};
		let (chart_ptr, chart_len) = slice_parts(&chart);
		let allocation = Box::new(BbAllocation {
			public: Self {
				filename: filename_view,
				assets,
				assets_len,
				desc: bkb_str {
					ptr: desc.as_ptr().cast(),
					len: desc.len(),
				},
				chart: chart_ptr,
				chart_len,
			},
			owner: BbOwner {
				inner,
				_views: views,
				_filename: filename,
				_desc: desc,
				_chart: chart,
			},
		});
		Box::into_raw(allocation).cast()
	}

	unsafe fn owner(&self) -> &BbOwner {
		let allocation = ptr::from_ref(self).cast::<BbAllocation>();
		unsafe { &(*allocation).owner }
	}

	pub(crate) unsafe fn inner(&self) -> &BackbeatFile {
		unsafe { &self.owner().inner }
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<BbAllocation>())) };
		}
	}
}

struct PackOwner {
	_views: CollectionViews,
	_bundles: Box<[bkb_pack_bundle]>,
}

#[repr(C)]
struct PackAllocation {
	public: bkb_pack,
	_owner: PackOwner,
}

const _: () = assert!(std::mem::offset_of!(PackAllocation, public) == 0);

impl bkb_pack {
	pub(crate) fn alloc(inner: PackContents) -> *mut Self {
		let mut views = CollectionViews::default();
		let name = views.string(&inner.name);
		let gamemode = views.string(inner.gamemode.as_str());
		let updated = timestamp(
			inner.updated.timestamp(),
			inner.updated.timestamp_subsec_nanos(),
		);
		let (tags, tags_len) = views.tags(inner.tags.iter());
		let (assets, assets_len) = views.assets(&inner.assets);
		let bundles = inner
			.bundles
			.iter()
			.map(|bundle| {
				let (tags, tags_len) = views.tags(bundle.tags.iter());
				bkb_pack_bundle {
					id: views.string(bundle.id.to_string()),
					desc: views.string(&bundle.desc),
					tags,
					tags_len,
					installed: bundle.installed,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let (bundles_ptr, bundles_len) = slice_parts(&bundles);
		let owner = PackOwner {
			_views: views,
			_bundles: bundles,
		};
		let allocation = Box::new(PackAllocation {
			public: Self {
				name,
				gamemode,
				updated,
				tags,
				tags_len,
				assets,
				assets_len,
				bundles: bundles_ptr,
				bundles_len,
			},
			_owner: owner,
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<PackAllocation>())) };
		}
	}
}

struct CourseOwner {
	_views: CollectionViews,
	_charts: Box<[bkb_course_chart]>,
}

#[repr(C)]
struct CourseAllocation {
	public: bkb_course,
	_owner: CourseOwner,
}

const _: () = assert!(std::mem::offset_of!(CourseAllocation, public) == 0);

impl bkb_course {
	pub(crate) fn alloc(inner: CourseContents) -> *mut Self {
		let mut views = CollectionViews::default();
		let name = views.string(&inner.name);
		let updated = timestamp(
			inner.updated.timestamp(),
			inner.updated.timestamp_subsec_nanos(),
		);
		let gamemode = views.string(inner.gamemode.as_str());
		let (tags, tags_len) = views.tags(inner.tags.iter());
		let (assets, assets_len) = views.assets(&inner.assets);
		let charts = inner
			.charts
			.iter()
			.map(|chart| {
				let (tags, tags_len) = views.tags(chart.tags.iter());
				let bundle_id = chart
					.bundle_id
					.map(|value| views.string(value.to_string()))
					.unwrap_or_else(null_str);
				bkb_course_chart {
					id: views.string(chart.id.to_string()),
					desc: views.string(&chart.desc),
					tags,
					tags_len,
					bundle_id,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let (charts_ptr, charts_len) = slice_parts(&charts);
		let owner = CourseOwner {
			_views: views,
			_charts: charts,
		};
		let allocation = Box::new(CourseAllocation {
			public: Self {
				name,
				updated,
				gamemode,
				tags,
				tags_len,
				assets,
				assets_len,
				charts: charts_ptr,
				charts_len,
			},
			_owner: owner,
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<CourseAllocation>())) };
		}
	}
}

pub(crate) struct TableOwner {
	_views: CollectionViews,
	_chart_arrays: Vec<Box<[bkb_table_chart]>>,
	_levels: Box<[bkb_table_level]>,
	_folder_chart_arrays: Vec<Box<[bkb_table_chart]>>,
	_folders: Box<[bkb_table_folder]>,
}

#[repr(C)]
struct TableAllocation {
	public: bkb_table,
	owner: TableOwner,
}

const _: () = assert!(std::mem::offset_of!(TableAllocation, public) == 0);

impl bkb_table {
	pub(crate) fn alloc(inner: TableContents) -> *mut Self {
		let mut views = CollectionViews::default();
		let name = views.string(&inner.name);
		let symbol = views.string(&inner.symbol);
		let gamemode = views.string(inner.gamemode.as_str());
		let updated = timestamp(
			inner.updated.timestamp(),
			inner.updated.timestamp_subsec_nanos(),
		);
		let (tags, tags_len) = views.tags(inner.tags.iter());
		let (assets, assets_len) = views.assets(&inner.assets);
		let mut chart_arrays = Vec::with_capacity(inner.levels.len());
		let mut levels = Vec::with_capacity(inner.levels.len());
		for level in &inner.levels {
			let (tags, tags_len) = views.tags(level.tags.iter());
			let charts = level
				.charts
				.iter()
				.map(|chart| views.table_chart(chart))
				.collect::<Vec<_>>()
				.into_boxed_slice();
			let (charts_ptr, charts_len) = slice_parts(&charts);
			chart_arrays.push(charts);
			levels.push(bkb_table_level {
				level: views.string(&level.level),
				tags,
				tags_len,
				charts: charts_ptr,
				charts_len,
			});
		}
		let levels = levels.into_boxed_slice();
		let (levels_ptr, levels_len) = slice_parts(&levels);
		let mut folder_chart_arrays = Vec::with_capacity(inner.folders.len());
		let mut folders = Vec::with_capacity(inner.folders.len());
		for folder in &inner.folders {
			let (tags, tags_len) = views.tags(folder.tags.iter());
			let chart_array = folder
				.charts
				.iter()
				.map(|chart| views.table_chart(chart))
				.collect::<Vec<_>>()
				.into_boxed_slice();
			let (charts, charts_len) = slice_parts(&chart_array);
			folder_chart_arrays.push(chart_array);
			folders.push(bkb_table_folder {
				name: views.string(&folder.name),
				query: views.string(&folder.query),
				tags,
				tags_len,
				charts,
				charts_len,
			});
		}
		let folders = folders.into_boxed_slice();
		let (folders_ptr, folders_len) = slice_parts(&folders);
		let owner = TableOwner {
			_views: views,
			_chart_arrays: chart_arrays,
			_levels: levels,
			_folder_chart_arrays: folder_chart_arrays,
			_folders: folders,
		};
		let allocation = Box::new(TableAllocation {
			public: Self {
				name,
				symbol,
				gamemode,
				updated,
				tags,
				tags_len,
				assets,
				assets_len,
				levels: levels_ptr,
				levels_len,
				folders: folders_ptr,
				folders_len,
			},
			owner,
		});
		Box::into_raw(allocation).cast()
	}

	pub(crate) unsafe fn free(value: *mut Self) {
		if !value.is_null() {
			unsafe { drop(Box::from_raw(value.cast::<TableAllocation>())) };
		}
	}
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_server_config {
	pub url: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct bkb_string {
	pub ptr: *mut c_char,
	pub len: usize,
}

impl bkb_string {
	pub(crate) fn new(value: String) -> Result<Self, bkb_error_code> {
		let len = value.len();
		let value = CString::new(value).map_err(|_| BKB_ERR_INVALID_STRING)?;
		Ok(Self {
			ptr: value.into_raw(),
			len,
		})
	}

	pub(crate) unsafe fn write(out: *mut Self, value: String) -> Result<(), bkb_error_code> {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let value = Self::new(value)?;
		unsafe { out.write(value) };
		Ok(())
	}
}

#[derive(Clone, Copy)]
#[repr(C)]
/// Library-owned bytes. `ptr` is null when `len` is zero.
pub struct bkb_bytes {
	pub ptr: *mut u8,
	pub len: usize,
}

impl bkb_bytes {
	pub(crate) fn new(value: Vec<u8>) -> Self {
		let len = value.len();
		if value.is_empty() {
			return Self {
				ptr: ptr::null_mut(),
				len,
			};
		}
		let ptr = Box::into_raw(value.into_boxed_slice()).cast::<u8>();
		Self { ptr, len }
	}

	pub(crate) unsafe fn write(out: *mut Self, value: Vec<u8>) -> Result<(), bkb_error_code> {
		if out.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		unsafe { out.write(Self::new(value)) };
		Ok(())
	}
}

pub(crate) unsafe fn cstr_to_str<'a>(value: *const c_char) -> Result<&'a str, bkb_error_code> {
	if value.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	unsafe { CStr::from_ptr(value) }
		.to_str()
		.map_err(|_| BKB_ERR_INVALID_STRING)
}

pub(crate) unsafe fn bytes_from_raw<'a>(
	value: *const u8,
	len: usize,
) -> Result<&'a [u8], bkb_error_code> {
	if value.is_null() {
		return Err(BKB_ERR_NULL_ARG);
	}
	Ok(unsafe { slice::from_raw_parts(value, len) })
}
