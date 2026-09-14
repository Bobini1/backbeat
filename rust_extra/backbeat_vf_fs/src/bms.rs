#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use backbeat_core::{AssetId, ChartFilename, Sha256};
use backbeat_core::{Assets, CombinedAssetsId};
use backbeat_sdk::Backbeat;
use futures::TryStreamExt;

use crate::database::open_readonly_database;
use crate::jails;
use crate::safe_paths::{MAX_FOLDER_NAME_CHARS, sanitize_folder_name, truncate_folder_name};
use crate::{BackbeatFilesystem, IndexError, MemDentryIndex, PathIndex};

#[derive(Debug)]
pub(crate) struct StoredBmsChart {
	pub filename: ChartFilename,
	pub sha256: Sha256,
	pub size: u64,
	pub assets: Assets,
	pub asset_sizes: HashMap<AssetId, Option<u64>>,
}

#[derive(Debug)]
pub(crate) struct BmsChartCandidate {
	pub chart: StoredBmsChart,
	pub combined_assets_id: CombinedAssetsId,
	pub description: String,
}

#[derive(Debug)]
pub(crate) struct BmsChartGroup {
	pub combined_assets_id: CombinedAssetsId,
	pub description: String,
	pub charts: Vec<BmsChartCandidate>,
}

#[derive(Debug)]
pub(crate) struct BmsFolder {
	pub name: String,
	pub group: BmsChartGroup,
}

/// Build the backend-neutral filesystem for the BMS virtual-folder prefab.
pub async fn build_filesystem(store: Backbeat) -> Result<BackbeatFilesystem, IndexError> {
	let index = build_index(&store).await?;
	let index = Arc::new(MemDentryIndex::new(index));
	Ok(BackbeatFilesystem::new(index, store))
}

async fn build_index(store: &Backbeat) -> Result<PathIndex, IndexError> {
	let candidates = load_chart_candidates(store).await?;
	let groups = group_chart_candidates(candidates);
	let folders = folder_groups(groups);
	let index = build_index_from_folders(folders)?;
	Ok(index)
}

async fn load_chart_candidates(store: &Backbeat) -> Result<Vec<BmsChartCandidate>, IndexError> {
	let pool = open_readonly_database(store).await?;

	let mut candidates = Vec::new();
	let mut representative_indexes = HashMap::new();
	let mut seen_assets = HashSet::new();
	let mut chart_rows = sqlx::query!(
		r#"
		SELECT
			b.id AS "bundle_id: backbeat_core::BundleId",
			b.filename AS "filename: backbeat_core::ChartFilename",
			b.chart_sha256 AS "sha256!: backbeat_core::Sha256",
			b.description AS "description!: String",
			b.combined_assets_id AS "combined_assets_id!: backbeat_core::CombinedAssetsId",
			cd.uncompressed_size AS "size!: i64"
		FROM bundle b
		JOIN chart_data cd ON cd.sha256 = b.chart_sha256
		WHERE b.extension IN ('bms', 'bme', 'bml', 'bmson')
		  AND b.combined_assets_id IS NOT NULL
		ORDER BY b.id
		"#,
	)
	.fetch(&pool);
	while let Some(row) = chart_rows.try_next().await? {
		let candidate_index = candidates.len();
		if seen_assets.insert(row.combined_assets_id) {
			representative_indexes.insert(row.bundle_id, candidate_index);
		}
		candidates.push(BmsChartCandidate {
			chart: StoredBmsChart {
				filename: row.filename,
				sha256: row.sha256,
				size: row.size.max(0) as u64,
				assets: Assets::new(),
				asset_sizes: HashMap::new(),
			},
			combined_assets_id: row.combined_assets_id,
			description: row.description,
		});
	}
	drop(chart_rows);

	let mut asset_rows = sqlx::query!(
		r#"
		WITH representatives AS (
			SELECT MIN(id) AS bundle_id
			FROM bundle
			WHERE extension IN ('bms', 'bme', 'bml', 'bmson')
			  AND combined_assets_id IS NOT NULL
			GROUP BY combined_assets_id
		)
		SELECT
			representatives.bundle_id AS "bundle_id!: backbeat_core::BundleId",
			ba.path AS "path: backbeat_core::AssetPath",
			ba.sha256 AS "asset_id!: backbeat_core::AssetId",
			da.size AS "size!: i64"
		FROM representatives
		JOIN bundle b ON b.id = representatives.bundle_id
		JOIN asset_map ba ON ba.combined_assets_id = b.combined_assets_id
		JOIN downloaded_asset da ON da.sha256 = ba.sha256
		"#,
	)
	.fetch(&pool);
	while let Some(row) = asset_rows.try_next().await? {
		let candidate = &mut candidates[*representative_indexes
			.get(&row.bundle_id)
			.expect("BMS asset row must belong to a loaded BMS bundle")];
		candidate.chart.assets.insert(row.path, row.asset_id);
		candidate
			.chart
			.asset_sizes
			.insert(row.asset_id, Some(row.size.max(0) as u64));
	}

	Ok(candidates)
}

fn group_chart_candidates(candidates: Vec<BmsChartCandidate>) -> Vec<BmsChartGroup> {
	let mut charts_by_assets = HashMap::<CombinedAssetsId, Vec<BmsChartCandidate>>::new();
	for candidate in candidates {
		charts_by_assets
			.entry(candidate.combined_assets_id)
			.or_default()
			.push(candidate);
	}

	let mut groups: Vec<_> = charts_by_assets
		.into_iter()
		.map(|(combined_assets_id, charts)| BmsChartGroup {
			combined_assets_id,
			description: consensus_description(&charts),
			charts,
		})
		.collect();
	groups.sort_by_key(|group| group.combined_assets_id.to_string());
	groups
}

fn consensus_description(charts: &[BmsChartCandidate]) -> String {
	let mut counts = std::collections::BTreeMap::<String, usize>::new();
	for chart in charts {
		if !chart.description.trim().is_empty() {
			*counts.entry(chart.description.clone()).or_default() += 1;
		}
	}

	counts
		.into_iter()
		.min_by(|(title_a, count_a), (title_b, count_b)| {
			count_b.cmp(count_a).then_with(|| title_a.cmp(title_b))
		})
		.map(|(description, _)| description)
		.unwrap_or_else(|| "Unknown Title".to_owned())
}

fn folder_groups(groups: Vec<BmsChartGroup>) -> Vec<BmsFolder> {
	let mut title_counts = HashMap::<String, usize>::new();
	for group in &groups {
		*title_counts.entry(folder_title(group)).or_default() += 1;
	}

	let mut folders: Vec<_> = groups
		.into_iter()
		.map(|group| {
			let title = folder_title(&group);
			let name = if title_counts[&title] > 1 {
				let suffix = format!(" ({})", &group.combined_assets_id.0.to_string()[..6]);
				format!(
					"{}{suffix}",
					truncate_folder_name(&title, MAX_FOLDER_NAME_CHARS - suffix.chars().count())
				)
			} else {
				truncate_folder_name(&title, MAX_FOLDER_NAME_CHARS)
			};
			BmsFolder { name, group }
		})
		.collect();
	folders.sort_by(|a, b| {
		a.name.cmp(&b.name).then_with(|| {
			a.group
				.combined_assets_id
				.0
				.cmp(&b.group.combined_assets_id.0)
		})
	});
	folders
}

fn folder_title(group: &BmsChartGroup) -> String {
	truncate_folder_name(
		&sanitize_folder_name(&group.description, "Unknown Title"),
		MAX_FOLDER_NAME_CHARS,
	)
}

fn build_index_from_folders(folders: Vec<BmsFolder>) -> Result<PathIndex, IndexError> {
	let mut index = PathIndex::new();
	for folder in folders {
		let jail = folder
			.group
			.charts
			.first()
			.map(|chart| jails::path(jails::depth(&chart.chart.assets)))
			.unwrap_or_default();
		for candidate in &folder.group.charts {
			let chart_path = format!(
				"{folder}/{jail}{filename}",
				folder = folder.name,
				filename = candidate.chart.filename
			);
			index.insert_chart(&chart_path, candidate.chart.sha256, candidate.chart.size)?;
		}

		if let Some(candidate) = folder.group.charts.first() {
			for (asset_path, asset_id) in &candidate.chart.assets {
				let Some(Some(size)) = candidate.chart.asset_sizes.get(asset_id) else {
					continue;
				};
				index.insert_asset(
					&format!("{folder}/{jail}{asset_path}", folder = folder.name),
					*asset_id,
					*size,
				)?;
			}
		}
	}
	Ok(index)
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::AssetPath;
	use backbeat_core::{BackbeatFile, ChartData, ChartDesc};
	use backbeat_store_config::BackbeatConfig;
	use tempfile::TempDir;

	use super::*;

	fn asset_path(value: &str) -> AssetPath {
		AssetPath::from_path(value).unwrap()
	}

	fn stored_chart(filename: &str) -> StoredBmsChart {
		StoredBmsChart {
			filename: ChartFilename::from_path(filename).expect("filename"),
			sha256: Sha256::null(),
			size: 0,
			assets: Assets::new(),
			asset_sizes: HashMap::new(),
		}
	}

	#[test]
	fn candidates_use_the_bundle_description() {
		let candidate = candidate(Some("Cached Title"), Assets::new());

		assert_eq!(candidate.description, "Cached Title");
		assert_eq!(candidate.chart.filename.as_str(), "chart.bms");
	}

	fn candidate(title: Option<&str>, assets: Assets) -> BmsChartCandidate {
		let combined_assets_id = CombinedAssetsId::compute(&assets);
		BmsChartCandidate {
			chart: StoredBmsChart {
				filename: ChartFilename::from_path("chart.bms").expect("filename"),
				sha256: Sha256::null(),
				size: 0,
				assets,
				asset_sizes: HashMap::new(),
			},
			combined_assets_id,
			description: title.unwrap_or("Unknown Title").to_owned(),
		}
	}

	#[test]
	fn groups_charts_by_assets_and_selects_consensus_title() {
		let first_asset = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second_asset = AssetId::from(Sha256::checksum_bytes(b"second"));
		let first_assets = HashMap::from([(asset_path("sound.wav"), first_asset)]);
		let second_assets = HashMap::from([(asset_path("sound.wav"), second_asset)]);

		let groups = group_chart_candidates(vec![
			candidate(Some("Beta"), first_assets.clone()),
			candidate(Some("Alpha"), first_assets.clone()),
			candidate(Some("Alpha"), first_assets),
			candidate(Some("Other"), second_assets),
		]);

		assert_eq!(groups.len(), 2);
		let first_group = groups
			.iter()
			.find(|group| group.charts.len() == 3)
			.expect("first asset group");
		assert_eq!(first_group.description, "Alpha");

		let tied_group = group_chart_candidates(vec![
			candidate(Some("Beta"), HashMap::new()),
			candidate(Some("Alpha"), HashMap::new()),
		])
		.pop()
		.expect("tied group");
		assert_eq!(tied_group.description, "Alpha");
	}

	#[test]
	fn duplicate_folder_titles_include_an_combined_assets_id_suffix() {
		let first_asset = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second_asset = AssetId::from(Sha256::checksum_bytes(b"second"));
		let folders = folder_groups(group_chart_candidates(vec![
			candidate(
				Some("Shared Title"),
				HashMap::from([(asset_path("sound.wav"), first_asset)]),
			),
			candidate(
				Some("Shared Title"),
				HashMap::from([(asset_path("sound.wav"), second_asset)]),
			),
			candidate(Some("Unique Title"), HashMap::new()),
		]));

		assert_eq!(folders.len(), 3);
		let shared_names: Vec<_> = folders
			.iter()
			.filter_map(|folder| folder.name.strip_prefix("Shared Title ("))
			.map(|suffix| suffix.strip_suffix(')').expect("suffix terminator"))
			.collect();
		assert_eq!(shared_names.len(), 2);
		assert_ne!(shared_names[0], shared_names[1]);
		for suffix in shared_names {
			assert_eq!(suffix.len(), 6);
			assert!(
				suffix
					.chars()
					.all(|character| character.is_ascii_hexdigit())
			);
			assert_eq!(suffix, suffix.to_ascii_lowercase());
		}
		assert!(folders.iter().any(|folder| folder.name == "Unique Title"));
	}

	#[test]
	fn folder_names_are_cross_platform_safe_and_disambiguated_after_sanitising() {
		let first_asset = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second_asset = AssetId::from(Sha256::checksum_bytes(b"second"));
		let folders = folder_groups(group_chart_candidates(vec![
			candidate(
				Some("Song/Name"),
				HashMap::from([(asset_path("sound.wav"), first_asset)]),
			),
			candidate(
				Some("Song:Name"),
				HashMap::from([(asset_path("sound.wav"), second_asset)]),
			),
			candidate(Some("CON. "), HashMap::new()),
		]));

		let song_names: Vec<_> = folders
			.iter()
			.filter(|folder| folder.name.starts_with("Song-Name ("))
			.collect();
		assert_eq!(song_names.len(), 2);
		assert!(folders.iter().any(|folder| folder.name == "CON-"));
		assert!(folders.iter().all(|folder| {
			!folder.name.chars().any(|character| {
				matches!(
					character,
					'/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
				)
			})
		}));
	}

	#[test]
	fn folder_names_are_limited_to_100_characters() {
		let title = "a".repeat(MAX_FOLDER_NAME_CHARS + 1);
		let duplicate_title = format!("{title}b");
		let first = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second = AssetId::from(Sha256::checksum_bytes(b"second"));
		let folders = folder_groups(group_chart_candidates(vec![
			candidate(
				Some(&title),
				HashMap::from([(asset_path("sound.wav"), first)]),
			),
			candidate(
				Some(&duplicate_title),
				HashMap::from([(asset_path("sound.wav"), second)]),
			),
		]));

		assert!(
			folders
				.iter()
				.all(|folder| folder.name.chars().count() <= MAX_FOLDER_NAME_CHARS)
		);
		assert_ne!(folders[0].name, folders[1].name);
	}

	#[test]
	fn jails_content_by_the_deepest_asset_up_reference() {
		let local = AssetId::from(Sha256::checksum_bytes(b"local"));
		let up_reference = AssetId::from(Sha256::checksum_bytes(b"up reference"));
		let assets = HashMap::from([
			(asset_path("local.wav"), local),
			(asset_path("../shared.wav"), up_reference),
		]);
		let mut chart = candidate(Some("Song"), assets.clone());
		chart.chart.asset_sizes = HashMap::from([(local, Some(1)), (up_reference, Some(1))]);

		let index = build_index_from_folders(folder_groups(group_chart_candidates(vec![chart])))
			.expect("build index");
		assert!(index.lookup_path("Song/_x/chart.bms").is_some());
		assert!(index.lookup_path("Song/_x/local.wav").is_some());
		assert!(index.lookup_path("Song/shared.wav").is_some());
	}

	#[test]
	fn group_assets_come_from_the_representative_chart() {
		let asset = AssetId::from(Sha256::checksum_bytes(b"asset"));
		let assets = HashMap::from([(asset_path("sound.wav"), asset)]);
		let mut representative = candidate(Some("Song"), assets);
		representative.chart.asset_sizes.insert(asset, Some(1));

		let mut other = candidate(Some("Song"), Assets::new());
		other.combined_assets_id = representative.combined_assets_id;
		other.chart.filename = ChartFilename::from_path("other.bms").expect("filename");

		let index = build_index_from_folders(folder_groups(group_chart_candidates(vec![
			representative,
			other,
		])))
		.expect("build index");

		assert!(index.lookup_path("Song/chart.bms").is_some());
		assert!(index.lookup_path("Song/other.bms").is_some());
		assert!(index.lookup_path("Song/sound.wav").is_some());
	}

	#[test]
	fn loads_only_bms_family_charts() {
		let temp = TempDir::new().expect("create temp directory");
		let config_dir = temp.path().join("config");
		let mut config = BackbeatConfig::default();
		config.store.path = temp.path().join("store");
		config
			.write_to_dir(&config_dir)
			.expect("write test configuration");
		let store = Backbeat::open_with_overridden_config_dir(&config_dir).expect("open store");

		let asset_bytes = b"asset data";
		let asset_id = AssetId::from(Sha256::checksum_bytes(asset_bytes));
		backbeat_sdk::test_support::store_asset_unchecked(&store, asset_id, asset_bytes)
			.expect("store asset");
		let missing_asset = AssetId::from(Sha256::checksum_bytes(b"missing asset"));
		let bms = BackbeatFile {
			filename: ChartFilename::from_path("test.BMS").expect("filename"),
			assets: HashMap::from([
				(asset_path("test.wav"), asset_id),
				(asset_path("missing.mp4"), missing_asset),
			]),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#TITLE BMS Title\n#ARTIST Backbeat\n")
				.expect("compress chart"),
		};
		let ksh = BackbeatFile {
			filename: ChartFilename::from_path("other.ksh").expect("filename"),
			assets: Assets::new(),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"title=KSH Title\nartist=Backbeat\n--\n--\n")
				.expect("compress chart"),
		};
		store.import_bundle(&bms).expect("import bms");
		store.import_bundle(&ksh).expect("import ksh");

		let runtime = tokio::runtime::Runtime::new().expect("create runtime");
		let candidates = runtime
			.block_on(load_chart_candidates(&store))
			.expect("load BMS candidates");

		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].description, "test");
		assert_eq!(
			candidates[0]
				.chart
				.assets
				.get(&AssetPath::from_path("test.wav").unwrap()),
			Some(&asset_id)
		);
		assert_eq!(
			candidates[0].chart.asset_sizes.get(&asset_id),
			Some(&Some(asset_bytes.len() as u64))
		);
		assert_eq!(candidates[0].chart.asset_sizes.get(&missing_asset), None);

		let index = runtime
			.block_on(build_index(&store))
			.expect("build BMS index");
		assert!(index.lookup_path("test/test.BMS").is_some());
		assert!(index.lookup_path("test/test.wav").is_some());
		assert!(index.lookup_path("test/missing.mp4").is_none());
	}
}
