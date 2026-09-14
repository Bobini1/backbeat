use std::collections::HashSet;

use backbeat_core::{
	Assets, BundleId, ChartId, CollectionKind, CourseChartTags, CourseTags, LevelTags,
	PackBundleTags, PackTags, TableChartTags, TableFolderTags, TableTags, ValidGamemodeIdentifier,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Sqlite;

use crate::store::{Backbeat, StoreError};
use crate::{DataId, Result};

/// Errors that can occur while fetching a Backbeat collection.
#[derive(Debug, thiserror::Error)]
pub enum CollectionClientError {
	#[error("request failed: {0}")]
	Request(#[from] reqwest::Error),

	#[error("HTTP request to {url} returned status {status}")]
	HttpStatus { url: String, status: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableContents {
	pub name: String,
	pub symbol: String,
	pub gamemode: ValidGamemodeIdentifier,
	pub updated: DateTime<Utc>,
	pub tags: TableTags,
	pub assets: Assets,
	pub levels: Vec<TableContentsLevel>,
	pub folders: Vec<TableContentsFolder>,
}

impl TableContents {
	pub fn charts(&self) -> impl Iterator<Item = (&TableContentsLevel, &TableContentsChart)> + '_ {
		self.levels
			.iter()
			.flat_map(|level| level.charts.iter().map(move |chart| (level, chart)))
	}

	pub fn chart_count(&self) -> usize {
		self.levels.iter().map(|level| level.charts.len()).sum()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableContentsLevel {
	pub level: String,
	pub tags: LevelTags,
	pub charts: Vec<TableContentsChart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableContentsChart {
	pub id: ChartId,
	pub desc: String,
	pub tags: TableChartTags,
	/// The bundle this chart ID resolved to in your store.
	/// if None, you don't have this bundle installed.
	pub bundle_id: Option<BundleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableContentsFolder {
	pub name: String,
	pub query: String,
	pub tags: TableFolderTags,

	pub charts: Vec<TableContentsChart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CourseContents {
	pub name: String,
	pub updated: DateTime<Utc>,
	pub gamemode: ValidGamemodeIdentifier,
	pub tags: CourseTags,
	pub assets: Assets,
	pub charts: Vec<CourseContentsChart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CourseContentsChart {
	pub id: ChartId,
	pub desc: String,
	pub tags: CourseChartTags,
	/// The bundle this chart ID resolved to in your store.
	/// if None, you don't have this bundle installed.
	pub bundle_id: Option<BundleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackContents {
	pub name: String,
	pub gamemode: ValidGamemodeIdentifier,
	pub updated: DateTime<Utc>,
	pub tags: PackTags,
	pub assets: Assets,
	pub bundles: Vec<PackContentsBundle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackContentsBundle {
	pub id: BundleId,
	pub desc: String,
	pub tags: PackBundleTags,
	pub installed: bool,
}

pub(crate) enum DownloadCollection {
	Table(TableContents),
	Course(CourseContents),
	Pack(PackContents),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectionDownloadDataReport {
	pub charts_downloaded: u64,
	pub charts_skipped: u64,
	pub charts_failed: u64,
	pub bundles_downloaded: u64,
	pub bundles_skipped: u64,
	pub bundles_failed: u64,
	pub assets_downloaded: u64,
	pub assets_skipped: u64,
	pub assets_failed: u64,
	pub errors: Vec<CollectionDownloadDataFailure>,
}

impl CollectionDownloadDataReport {
	pub fn has_failures(&self) -> bool {
		!self.errors.is_empty()
	}

	pub(crate) fn merge(&mut self, other: Self) {
		self.charts_downloaded += other.charts_downloaded;
		self.charts_skipped += other.charts_skipped;
		self.charts_failed += other.charts_failed;
		self.bundles_downloaded += other.bundles_downloaded;
		self.bundles_skipped += other.bundles_skipped;
		self.bundles_failed += other.bundles_failed;
		self.assets_downloaded += other.assets_downloaded;
		self.assets_skipped += other.assets_skipped;
		self.assets_failed += other.assets_failed;
		self.errors.extend(other.errors);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDownloadDataFailure {
	pub item: DataId,
	pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDownloadProgress {
	pub current: u64,
	pub total: u64,
	pub item: DataId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionUpsertResult {
	pub kind: CollectionKind,
	pub status: CollectionUpsertStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionUpsertStatus {
	Inserted,
	Updated,
	TimestampUnchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CollectionMetadata {
	pub url: String,
	pub name: String,
	pub gamemode: ValidGamemodeIdentifier,
	pub updated: DateTime<Utc>,
	pub installed: u64,
	pub total: u64,
}

pub(crate) async fn ensure_url_kind(
	tx: &mut sqlx::Transaction<'_, Sqlite>,
	url: &str,
	kind: CollectionKind,
) -> Result<()> {
	let table = sqlx::query!("SELECT url FROM difftable WHERE url = ?1", url)
		.fetch_optional(&mut **tx)
		.await?
		.is_some();
	let course = sqlx::query!("SELECT url FROM course WHERE url = ?1", url)
		.fetch_optional(&mut **tx)
		.await?
		.is_some();
	let pack = sqlx::query!("SELECT url FROM pack WHERE url = ?1", url)
		.fetch_optional(&mut **tx)
		.await?
		.is_some();

	// silly, but whatever
	let other = match kind {
		CollectionKind::Table => [
			(course, CollectionKind::Course),
			(pack, CollectionKind::Pack),
		],
		CollectionKind::Course => [(table, CollectionKind::Table), (pack, CollectionKind::Pack)],
		CollectionKind::Pack => [
			(table, CollectionKind::Table),
			(course, CollectionKind::Course),
		],
	};

	if let Some((_, existing)) = other.into_iter().find(|(present, _)| *present) {
		return Err(StoreError::Parse(format!(
			"collection URL {url:?} is already installed as {}",
			existing.as_str()
		)));
	}
	Ok(())
}

pub(crate) fn plan_download_tasks(collection: DownloadCollection) -> Vec<DataId> {
	let mut tasks = Vec::new();
	let mut seen_charts = HashSet::new();
	let mut seen_bundles = HashSet::new();
	let mut seen_assets = HashSet::new();

	match collection {
		DownloadCollection::Table(table) => {
			for (_, chart) in table.charts() {
				if seen_charts.insert(chart.id.to_string()) {
					tasks.push(DataId::Chart(chart.id.clone()));
				}
			}
			for asset_id in table.assets.into_values() {
				if seen_assets.insert(asset_id) {
					tasks.push(DataId::Asset(asset_id));
				}
			}
		}
		DownloadCollection::Course(course) => {
			for chart_id in course.charts.into_iter().map(|chart| chart.id) {
				if seen_charts.insert(chart_id.to_string()) {
					tasks.push(DataId::Chart(chart_id));
				}
			}
			for asset_id in course.assets.into_values() {
				if seen_assets.insert(asset_id) {
					tasks.push(DataId::Asset(asset_id));
				}
			}
		}
		DownloadCollection::Pack(pack) => {
			for bundle_id in pack.bundles.into_iter().map(|bundle| bundle.id) {
				if seen_bundles.insert(bundle_id) {
					tasks.push(DataId::Bundle(bundle_id));
				}
			}
			for asset_id in pack.assets.into_values() {
				if seen_assets.insert(asset_id) {
					tasks.push(DataId::Asset(asset_id));
				}
			}
		}
	}

	tasks
}

pub(crate) async fn run_download_task(
	store: &Backbeat,
	task: DataId,
	report: &mut CollectionDownloadDataReport,
) -> Result<()> {
	match &task {
		DataId::Chart(chart_id) => {
			if store.has_chart_async(chart_id).await? {
				report.charts_skipped += 1;
				return Ok(());
			}

			match store.server_download_chart(chart_id).await {
				Ok(()) => report.charts_downloaded += 1,
				Err(err) => {
					report.charts_failed += 1;
					report.errors.push(CollectionDownloadDataFailure {
						item: task.clone(),
						message: err.to_string(),
					});
				}
			}
		}
		DataId::Bundle(bundle_id) => {
			if store.has_bundle_async(*bundle_id).await? {
				report.bundles_skipped += 1;
				return Ok(());
			}

			match store.server_download_bundle(*bundle_id).await {
				Ok(()) => report.bundles_downloaded += 1,
				Err(err) => {
					report.bundles_failed += 1;

					report.errors.push(CollectionDownloadDataFailure {
						item: task,
						message: err.to_string(),
					});
				}
			}
		}
		DataId::Asset(asset_id) => {
			if store.has_asset_async(*asset_id).await? {
				report.assets_skipped += 1;
				return Ok(());
			}

			match store.server_download_asset(*asset_id).await {
				Ok(()) => report.assets_downloaded += 1,
				Err(err) => {
					report.assets_failed += 1;
					report.errors.push(CollectionDownloadDataFailure {
						item: task,
						message: err.to_string(),
					});
				}
			}
		}
	}
	Ok(())
}
