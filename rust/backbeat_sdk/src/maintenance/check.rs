//! Store integrity checking.
//!
//! [`Backbeat::corruption_check`] verifies that:
//!
//! 1. Every large (on-disk) asset file exists at its expected path.
//! 2. Every large asset file's SHA-256 matches its stored key.
//! 3. Every chart's stored SHA-256 matches a re-hash of its decompressed data.
//! 4. Every additional chart ID can be recomputed and matches the stored value.
//! 5. Every `asset_map` row references an asset that actually exists in
//!    `downloaded_asset`.

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;

use backbeat_core::{
	AssetId, Assets, BackbeatFile, BundleId, ChartData, ChartDesc, ChartId, CombinedAssetsId,
	Sha256,
};

use crate::Result;
use crate::store::Backbeat;
use crate::util::BLOCK;

/// A chart ID entry whose recomputed value differs from what is stored.
pub struct WrongChartId {
	pub chart_sha256: Sha256,
	pub alg: String,
	pub stored_id: String,
	pub computed_id: String,
}

/// A chart ID entry that could not be recomputed (e.g. chart parse error).
///
/// These are reported as warnings, not failures — the stored ID may still be
/// correct if the algorithm intentionally skips unparseable files.
pub struct UncomputedChartId {
	pub chart_sha256: Sha256,
	pub alg: String,
	pub reason: String,
}

/// A `asset_map` row referencing an asset that doesn't exist in
/// `downloaded_asset`.
pub struct DanglingAssetRef {
	pub bundle_id: String,
	pub path: String,
	pub asset_id: AssetId,
}

/// The result of a [`Backbeat::corruption_check`] call.
pub struct CorruptionReport {
	/// Number of large (on-disk) assets checked.
	pub large_asset_count: usize,
	/// Number of charts checked.
	pub chart_count: usize,
	/// Number of chart ID entries checked.
	pub chart_id_count: usize,
	/// Assets with `data IS NULL` where the file does not exist on disk.
	pub missing_assets: Vec<AssetId>,
	/// Assets whose inline or on-disk SHA-256 does not match the stored key.
	pub corrupt_assets: Vec<AssetId>,
	/// Charts whose stored SHA-256 is wrong, whose data could not be decompressed,
	/// or which could not be inspected.
	pub corrupt_charts: Vec<Sha256>,
	/// Chart ID entries that do not match when recomputed.
	pub wrong_chart_ids: Vec<WrongChartId>,
	/// Chart ID entries that could not be recomputed. These are warnings, not
	/// counted as failures by [`CorruptionReport::is_ok`].
	pub uncomputable_chart_ids: Vec<UncomputedChartId>,
	/// `asset_map` rows whose `sha256` has no matching row in
	/// `downloaded_asset` — a chart referencing an asset that was never
	/// (successfully) stored.
	pub dangling_asset_refs: Vec<DanglingAssetRef>,
}

pub(crate) struct BundleRepair {
	pub(crate) bundle_id: BundleId,
	pub(crate) chart_sha256: Sha256,
	pub(crate) description: ChartDesc,
}

pub(crate) struct ChartIdRepair {
	pub(crate) chart_sha256: Sha256,
	pub(crate) chart_ids: Vec<String>,
	pub(crate) needs_update: bool,
}

pub(crate) struct CorruptionScan {
	pub(crate) report: CorruptionReport,
	pub(crate) bundles: Vec<BundleRepair>,
	pub(crate) chart_ids: Vec<ChartIdRepair>,
}

impl Backbeat {
	pub(crate) fn corruption_scan(&self) -> Result<CorruptionScan> {
		let asset_rows = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					sha256 AS "asset_id!: backbeat_core::AssetId",
					inline_data
				FROM downloaded_asset
				ORDER BY sha256
				"#,
			)
			.fetch_all(&self.pool),
		)?;
		let mut large_asset_count = 0;
		let mut missing_assets = Vec::new();
		let mut corrupt_assets = Vec::new();
		for row in asset_rows {
			if let Some(data) = row.inline_data {
				if AssetId(Sha256::checksum_bytes(&data)) != row.asset_id {
					corrupt_assets.push(row.asset_id);
				}
				continue;
			}

			large_asset_count += 1;
			let path = self.assets.asset_path(row.asset_id);
			let file = match crate::fs::File::open(&path) {
				Ok(file) => file,
				Err(err) if err.kind() == ErrorKind::NotFound => {
					missing_assets.push(row.asset_id);
					continue;
				}
				Err(err) => return Err(err.into()),
			};
			if AssetId(Sha256::checksum_data(file)?) != row.asset_id {
				corrupt_assets.push(row.asset_id);
			}
		}

		let chart_rows = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					sha256 AS "chart_sha256!: backbeat_core::Sha256",
					gzip_data AS "data!: Vec<u8>"
				FROM chart_data
				ORDER BY sha256
				"#,
			)
			.fetch_all(&self.pool),
		)?;
		let chart_count = chart_rows.len();
		let mut charts = HashMap::new();
		let mut corrupt_charts = HashSet::new();
		for row in chart_rows {
			let Ok(chart) = ChartData::from_compressed(row.data) else {
				corrupt_charts.insert(row.chart_sha256);
				continue;
			};

			if Sha256::checksum_bytes(&chart.decompress()) != row.chart_sha256 {
				corrupt_charts.insert(row.chart_sha256);
				continue;
			}
			charts.insert(row.chart_sha256, chart);
		}

		let asset_map_rows = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					combined_assets_id AS "combined_assets_id!: backbeat_core::CombinedAssetsId",
					path AS "path!: backbeat_core::AssetPath",
					sha256 AS "asset_id!: backbeat_core::AssetId"
				FROM asset_map
				"#,
			)
			.fetch_all(&self.pool),
		)?;
		let mut asset_maps: HashMap<CombinedAssetsId, Assets> = HashMap::new();
		for row in asset_map_rows {
			asset_maps
				.entry(row.combined_assets_id)
				.or_default()
				.insert(row.path, row.asset_id);
		}

		let bundle_rows = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					id AS "bundle_id!: backbeat_core::BundleId",
					description AS "desc!: backbeat_core::ChartDesc",
					chart_sha256 AS "chart_sha256!: backbeat_core::Sha256",
					filename AS "filename!: backbeat_core::ChartFilename",
					combined_assets_id AS "combined_assets_id!: backbeat_core::CombinedAssetsId"
				FROM bundle
				ORDER BY id
				"#,
			)
			.fetch_all(&self.pool),
		)?;
		let mut bundle_chart_shas = HashSet::new();
		let mut bundles = Vec::new();
		let mut desired_chart_ids: HashMap<Sha256, HashSet<String>> = HashMap::new();
		for row in bundle_rows {
			bundle_chart_shas.insert(row.chart_sha256);
			let Some(chart) = charts.get(&row.chart_sha256) else {
				corrupt_charts.insert(row.chart_sha256);
				continue;
			};
			let bb = BackbeatFile {
				filename: row.filename,
				assets: asset_maps
					.get(&row.combined_assets_id)
					.cloned()
					.unwrap_or_default(),
				desc: row.desc,
				chart: chart.clone(),
			};

			let chart_ids = bb.additional_chart_ids();

			desired_chart_ids
				.entry(row.chart_sha256)
				.or_default()
				.extend(chart_ids.into_iter().map(|id| id.to_string()));
			bundles.push(BundleRepair {
				bundle_id: row.bundle_id,
				chart_sha256: row.chart_sha256,
				description: bb.desc,
			});
		}
		for chart_sha256 in charts.keys() {
			if !bundle_chart_shas.contains(chart_sha256) {
				corrupt_charts.insert(*chart_sha256);
			}
		}
		bundles.retain(|bundle| !corrupt_charts.contains(&bundle.chart_sha256));
		desired_chart_ids.retain(|chart_sha256, _| !corrupt_charts.contains(chart_sha256));

		let chart_id_rows = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					chart_sha256 AS "chart_sha256!: backbeat_core::Sha256",
					id AS "id!: String"
				FROM chart_id
				ORDER BY chart_sha256, id
				"#,
			)
			.fetch_all(&self.pool),
		)?;
		let chart_id_count = chart_id_rows.len();
		let mut stored_chart_ids: HashMap<Sha256, HashSet<String>> = HashMap::new();
		let mut wrong_chart_ids = Vec::new();
		let mut uncomputable_chart_ids = Vec::new();
		for row in chart_id_rows {
			stored_chart_ids
				.entry(row.chart_sha256)
				.or_default()
				.insert(row.id.clone());
			if corrupt_charts.contains(&row.chart_sha256) {
				continue;
			}
			let stored = match row.id.parse::<ChartId>() {
				Ok(id) => id,
				Err(err) => {
					uncomputable_chart_ids.push(UncomputedChartId {
						chart_sha256: row.chart_sha256,
						alg: "unknown".to_owned(),
						reason: format!("invalid stored chart ID {:?}: {err}", row.id),
					});
					continue;
				}
			};
			if desired_chart_ids
				.get(&row.chart_sha256)
				.is_some_and(|ids| ids.contains(&row.id))
			{
				continue;
			}
			let computed = desired_chart_ids
				.get(&row.chart_sha256)
				.into_iter()
				.flatten()
				.filter_map(|id| id.parse::<ChartId>().ok())
				.find(|id| id.alg == stored.alg);
			match computed {
				Some(computed) if computed != stored => wrong_chart_ids.push(WrongChartId {
					chart_sha256: row.chart_sha256,
					alg: stored.alg.to_string(),
					stored_id: stored.val,
					computed_id: computed.val,
				}),
				Some(_) => {}
				None => uncomputable_chart_ids.push(UncomputedChartId {
					chart_sha256: row.chart_sha256,
					alg: stored.alg.to_string(),
					reason: "inspector did not produce this algorithm".to_owned(),
				}),
			}
		}

		let mut chart_ids = Vec::new();
		for chart_sha256 in bundle_chart_shas {
			if corrupt_charts.contains(&chart_sha256) {
				continue;
			}
			let mut desired: Vec<_> = desired_chart_ids
				.remove(&chart_sha256)
				.unwrap_or_default()
				.into_iter()
				.collect();
			desired.sort_unstable();
			let stored = stored_chart_ids.remove(&chart_sha256).unwrap_or_default();
			let needs_update =
				desired.len() != stored.len() || desired.iter().any(|id| !stored.contains(id));
			chart_ids.push(ChartIdRepair {
				chart_sha256,
				chart_ids: desired,
				needs_update,
			});
		}

		let dangling_asset_refs = BLOCK(
			sqlx::query!(
				r#"
				SELECT
					bundle.id AS "bundle_id!: String",
					asset_map.path AS "path!: String",
					asset_map.sha256 AS "asset_id!: backbeat_core::AssetId"
				FROM bundle
				JOIN asset_map USING (combined_assets_id)
				LEFT JOIN downloaded_asset ON downloaded_asset.sha256 = asset_map.sha256
				WHERE downloaded_asset.sha256 IS NULL
				ORDER BY bundle.id, asset_map.path
				"#,
			)
			.fetch_all(&self.pool),
		)?
		.into_iter()
		.map(|row| DanglingAssetRef {
			bundle_id: row.bundle_id,
			path: row.path,
			asset_id: row.asset_id,
		})
		.collect();

		let mut corrupt_charts: Vec<_> = corrupt_charts.into_iter().collect();
		corrupt_charts.sort_unstable();
		Ok(CorruptionScan {
			report: CorruptionReport {
				large_asset_count,
				chart_count,
				chart_id_count,
				missing_assets,
				corrupt_assets,
				corrupt_charts,
				wrong_chart_ids,
				uncomputable_chart_ids,
				dangling_asset_refs,
			},
			bundles,
			chart_ids,
		})
	}
}

impl CorruptionReport {
	/// `true` if no integrity failures were detected (warnings excluded).
	pub fn is_ok(&self) -> bool {
		self.missing_assets.is_empty()
			&& self.corrupt_assets.is_empty()
			&& self.corrupt_charts.is_empty()
			&& self.wrong_chart_ids.is_empty()
			&& self.dangling_asset_refs.is_empty()
	}

	/// Total number of integrity failures (warnings excluded).
	pub fn issue_count(&self) -> usize {
		self.missing_assets.len()
			+ self.corrupt_assets.len()
			+ self.corrupt_charts.len()
			+ self.wrong_chart_ids.len()
			+ self.dangling_asset_refs.len()
	}
}
