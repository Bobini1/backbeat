use std::collections::btree_map::Entry as BTreeEntry;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path};
use std::sync::Arc;

use backbeat_core::collections::PackBundleTags;
use backbeat_core::{AssetId, BundleId, ChartFilename, Sha256};
use backbeat_core::{Assets, CombinedAssetsId};
use backbeat_sdk::Backbeat;
use futures::TryStreamExt;

use crate::database::open_readonly_database;
use crate::safe_paths::{MAX_FOLDER_NAME_CHARS, sanitize_folder_name, truncate_folder_name};
use crate::{BackbeatFilesystem, IndexError, MemDentryIndex, PathIndex};

#[derive(Debug, Clone)]
struct StoredStepmaniaChart {
	bundle_id: BundleId,
	filename: ChartFilename,
	sha256: Sha256,
	size: u64,
}

#[derive(Debug, Clone)]
struct PackMembership {
	pack_name: String,
	song_folder: Option<String>,
}

#[derive(Debug, Clone)]
struct StepmaniaChartCandidate {
	chart: StoredStepmaniaChart,
	combined_assets_id: CombinedAssetsId,
	description: String,
	memberships: Vec<PackMembership>,
}

#[derive(Debug, Default)]
struct AssetSet {
	assets: Assets,
	sizes: HashMap<AssetId, u64>,
}

#[derive(Debug)]
struct PackAsset {
	pack_name: String,
	path: String,
	asset_id: AssetId,
	size: u64,
}

#[derive(Debug, Default)]
struct StepmaniaCharts {
	candidates: Vec<StepmaniaChartCandidate>,
	assets: HashMap<CombinedAssetsId, AssetSet>,
	pack_assets: Vec<PackAsset>,
}

#[derive(Debug, Clone)]
struct StepmaniaFolder {
	parent: String,
	base_name: String,
	name: String,
	combined_assets_id: CombinedAssetsId,
	charts: Vec<StoredStepmaniaChart>,
}

/// Build the backend-neutral filesystem for the StepMania virtual-folder prefab.
pub async fn build_filesystem(store: Backbeat) -> Result<BackbeatFilesystem, IndexError> {
	let index = build_index(&store).await?;
	let index = Arc::new(MemDentryIndex::new(index));
	Ok(BackbeatFilesystem::new(index, store))
}

async fn build_index(store: &Backbeat) -> Result<PathIndex, IndexError> {
	let charts = load_chart_candidates(store).await?;
	let folders = folder_chart_candidates(charts.candidates);
	build_index_from_folders(folders, charts.assets, charts.pack_assets)
}

async fn load_chart_candidates(store: &Backbeat) -> Result<StepmaniaCharts, IndexError> {
	let pool = open_readonly_database(store).await?;
	let mut charts = StepmaniaCharts::default();
	let mut candidate_indexes = HashMap::new();
	let mut representative_ids = HashMap::new();

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
		WHERE b.extension IN ('sm', 'ssc', 'dwi')
		  AND b.combined_assets_id IS NOT NULL
		ORDER BY b.id
		"#,
	)
	.fetch(&pool);
	while let Some(row) = chart_rows.try_next().await? {
		let candidate_index = charts.candidates.len();
		candidate_indexes.insert(row.bundle_id, candidate_index);
		if let Entry::Vacant(entry) = charts.assets.entry(row.combined_assets_id) {
			representative_ids.insert(row.bundle_id, row.combined_assets_id);
			entry.insert(AssetSet::default());
		}
		charts.candidates.push(StepmaniaChartCandidate {
			chart: StoredStepmaniaChart {
				bundle_id: row.bundle_id,
				filename: row.filename,
				sha256: row.sha256,
				size: row.size.max(0) as u64,
			},
			combined_assets_id: row.combined_assets_id,
			description: row.description,
			memberships: Vec::new(),
		});
	}
	drop(chart_rows);

	warn_duplicate_pack_names(&pool).await?;
	let mut membership_rows = sqlx::query!(
		r#"
		SELECT
			pe.bundle_id AS "bundle_id: backbeat_core::BundleId",
			p.name AS pack_name,
			pe.tags AS tags
		FROM
			pack_entry pe
		JOIN
			pack p ON p.url = pe.url
		JOIN
			bundle b ON b.id = pe.bundle_id
		WHERE b.extension IN ('sm', 'ssc', 'dwi')
		ORDER BY
			p.url, pe.entry
		"#,
	)
	.fetch(&pool);
	while let Some(row) = membership_rows.try_next().await? {
		let Some(&candidate_index) = candidate_indexes.get(&row.bundle_id) else {
			continue;
		};
		let tags: PackBundleTags =
			serde_json::from_str(&row.tags).map_err(|_| IndexError::InvalidValue {
				field: "pack entry tags",
				value: row.tags,
			})?;
		let song_folder = tags
			.get("sm/song-folder")
			.filter(|name| !name.trim().is_empty())
			.cloned();
		charts.candidates[candidate_index]
			.memberships
			.push(PackMembership {
				pack_name: row.pack_name,
				song_folder,
			});
	}
	drop(membership_rows);

	let mut asset_rows = sqlx::query!(
		r#"
		WITH representatives AS (
			SELECT MIN(id) AS bundle_id
			FROM bundle
			WHERE extension IN ('sm', 'ssc', 'dwi')
			  AND combined_assets_id IS NOT NULL
			GROUP BY combined_assets_id
		)
		SELECT
			representatives.bundle_id AS "bundle_id!: backbeat_core::BundleId",
			ba.path AS "path: backbeat_core::AssetPath",
			ba.sha256 AS "asset_id!: backbeat_core::AssetId",
			da.size AS "size!: i64"
		FROM
			representatives
		JOIN
			bundle b ON b.id = representatives.bundle_id
		JOIN
			asset_map ba ON ba.combined_assets_id = b.combined_assets_id
		JOIN
			downloaded_asset da ON da.sha256 = ba.sha256
		"#,
	)
	.fetch(&pool);
	while let Some(row) = asset_rows.try_next().await? {
		let combined_assets_id = representative_ids
			.get(&row.bundle_id)
			.expect("StepMania asset row must belong to a loaded StepMania bundle");
		let assets = charts
			.assets
			.get_mut(combined_assets_id)
			.expect("StepMania asset set must exist");
		assets.assets.insert(row.path, row.asset_id);
		assets.sizes.insert(row.asset_id, row.size.max(0) as u64);
	}
	drop(asset_rows);

	let mut pack_asset_rows = sqlx::query!(
		r#"
		SELECT
			p.name AS pack_name,
			pa.path AS path,
			pa.sha256 AS "asset_id!: backbeat_core::AssetId",
			da.size AS "size!: i64"
		FROM pack p
		JOIN pack_asset pa ON pa.url = p.url
		JOIN downloaded_asset da ON da.sha256 = pa.sha256
		WHERE EXISTS (
			SELECT 1
			FROM pack_entry pe
			JOIN bundle b ON b.id = pe.bundle_id
			WHERE pe.url = p.url
			  AND b.extension IN ('sm', 'ssc', 'dwi')
		)
		"#,
	)
	.fetch(&pool);
	while let Some(row) = pack_asset_rows.try_next().await? {
		charts.pack_assets.push(PackAsset {
			pack_name: row.pack_name,
			path: row.path,
			asset_id: row.asset_id,
			size: row.size.max(0) as u64,
		});
	}

	Ok(charts)
}

async fn warn_duplicate_pack_names(pool: &sqlx::SqlitePool) -> Result<(), IndexError> {
	let rows = sqlx::query!(
		r#"
		SELECT p.url, p.name
		FROM pack p
		WHERE EXISTS (
			SELECT 1
			FROM pack_entry pe
			JOIN bundle b ON b.id = pe.bundle_id
			WHERE pe.url = p.url
			  AND b.extension IN ('sm', 'ssc', 'dwi')
		)
		"#
	)
	.fetch_all(pool)
	.await?;
	let mut urls_by_name = BTreeMap::<String, Vec<String>>::new();
	for row in rows {
		urls_by_name
			.entry(pack_folder_name(&row.name))
			.or_default()
			.push(row.url);
	}
	for (name, mut urls) in urls_by_name {
		if urls.len() > 1 {
			urls.sort();
			tracing::warn!(pack_name = %name, ?urls, "multiple installed StepMania packs share a folder name");
		}
	}
	Ok(())
}

fn folder_chart_candidates(candidates: Vec<StepmaniaChartCandidate>) -> Vec<StepmaniaFolder> {
	let metadata = consensus_metadata(&candidates);
	let mut folders = HashMap::<(String, String, CombinedAssetsId), StepmaniaFolder>::new();

	for candidate in candidates {
		let fallback_name = metadata
			.get(&candidate.combined_assets_id)
			.expect("StepMania chart must have consensus metadata");
		let placements = if candidate.memberships.is_empty() {
			vec![("_Backbeat Other".to_owned(), fallback_name.clone())]
		} else {
			candidate
				.memberships
				.iter()
				.map(|membership| {
					let name = membership
						.song_folder
						.as_deref()
						.map(song_folder_name)
						.unwrap_or_else(|| fallback_name.clone());
					(pack_folder_name(&membership.pack_name), name)
				})
				.collect()
		};

		for (parent, base_name) in placements {
			let folder = folders
				.entry((
					parent.clone(),
					base_name.clone(),
					candidate.combined_assets_id,
				))
				.or_insert_with(|| StepmaniaFolder {
					parent,
					base_name: base_name.clone(),
					name: base_name,
					combined_assets_id: candidate.combined_assets_id,
					charts: Vec::new(),
				});
			folder.charts.push(candidate.chart.clone());
		}
	}

	let mut counts = HashMap::<(String, String), usize>::new();
	for folder in folders.values() {
		*counts
			.entry((folder.parent.clone(), folder.base_name.clone()))
			.or_default() += 1;
	}

	let mut folders: Vec<_> = folders
		.into_values()
		.map(|mut folder| {
			if counts[&(folder.parent.clone(), folder.base_name.clone())] > 1 {
				let suffix = format!(" ({})", &folder.combined_assets_id.0.to_string()[..6]);
				folder.name = format!(
					"{}{suffix}",
					truncate_folder_name(
						&folder.base_name,
						MAX_FOLDER_NAME_CHARS - suffix.chars().count()
					)
				);
			}
			folder
		})
		.collect();
	folders.sort_by(|a, b| {
		a.parent
			.cmp(&b.parent)
			.then_with(|| a.name.cmp(&b.name))
			.then_with(|| a.combined_assets_id.0.cmp(&b.combined_assets_id.0))
	});
	folders
}

fn consensus_metadata(candidates: &[StepmaniaChartCandidate]) -> HashMap<CombinedAssetsId, String> {
	let mut fields = HashMap::<CombinedAssetsId, Vec<String>>::new();
	for candidate in candidates {
		fields
			.entry(candidate.combined_assets_id)
			.or_default()
			.push(candidate.description.clone());
	}
	fields
		.into_iter()
		.map(|(combined_assets_id, descriptions)| {
			let description =
				consensus_field(&descriptions).unwrap_or_else(|| "Unknown Title".to_owned());
			(combined_assets_id, song_folder_name(&description))
		})
		.collect()
}

fn consensus_field(values: &[String]) -> Option<String> {
	let mut counts = BTreeMap::<String, usize>::new();
	for value in values {
		if !value.trim().is_empty() {
			*counts.entry(value.clone()).or_default() += 1;
		}
	}
	counts
		.into_iter()
		.min_by(|(value_a, count_a), (value_b, count_b)| {
			count_b.cmp(count_a).then_with(|| value_a.cmp(value_b))
		})
		.map(|(value, _)| value)
}

fn pack_folder_name(name: &str) -> String {
	truncate_folder_name(
		&sanitize_folder_name(name, "Unknown Pack"),
		MAX_FOLDER_NAME_CHARS,
	)
}

fn song_folder_name(name: &str) -> String {
	truncate_folder_name(
		&sanitize_folder_name(name, "Unknown Title"),
		MAX_FOLDER_NAME_CHARS,
	)
}

fn build_index_from_folders(
	folders: Vec<StepmaniaFolder>,
	assets_by_id: HashMap<CombinedAssetsId, AssetSet>,
	pack_assets: Vec<PackAsset>,
) -> Result<PathIndex, IndexError> {
	let mut index = PathIndex::new();
	let mut asset_files = BTreeMap::<String, (AssetId, u64)>::new();
	let mut chart_files = BTreeMap::<String, Vec<StoredStepmaniaChart>>::new();
	for folder in folders {
		let assets = assets_by_id
			.get(&folder.combined_assets_id)
			.expect("StepMania folder must have an asset set");
		let folder_path = format!("{}/{}/", folder.parent, folder.name);
		for chart in &folder.charts {
			chart_files
				.entry(format!("{folder_path}{}", chart.filename))
				.or_default()
				.push(chart.clone());
		}
		for (asset_path, asset_id) in &assets.assets {
			let size = assets.sizes[asset_id];
			let path = resolve_asset_path(&format!("{folder_path}{asset_path}"))?;
			insert_asset_file(&mut asset_files, path, *asset_id, size);
		}
	}
	for asset in pack_assets {
		let path = resolve_asset_path(&format!(
			"{}/{path}",
			pack_folder_name(&asset.pack_name),
			path = asset.path
		))?;
		insert_asset_file(&mut asset_files, path, asset.asset_id, asset.size);
	}
	for (path, charts) in chart_files {
		if charts.len() == 1 {
			let chart = charts.into_iter().next().expect("one chart");
			index.insert_chart(&path, chart.sha256, chart.size)?;
			continue;
		}

		let filename = &charts[0].filename;
		if !backbeat_packager::supports_melding(filename) {
			return Err(IndexError::UnmeldableChartCollision {
				path,
				filename: filename.to_string(),
			});
		}
		index.insert_melded_chart(
			&path,
			charts
				.into_iter()
				.map(|chart| chart.bundle_id)
				.collect::<Vec<_>>(),
		)?;
	}
	for (path, (asset_id, size)) in asset_files {
		index.insert_asset(&path, asset_id, size)?;
	}
	Ok(index)
}

fn insert_asset_file(
	asset_files: &mut BTreeMap<String, (AssetId, u64)>,
	path: String,
	asset_id: AssetId,
	size: u64,
) {
	match asset_files.entry(path) {
		BTreeEntry::Vacant(entry) => {
			entry.insert((asset_id, size));
		}
		BTreeEntry::Occupied(mut entry) if asset_id.0 < entry.get().0.0 => {
			entry.insert((asset_id, size));
		}
		BTreeEntry::Occupied(_) => {}
	}
}

fn resolve_asset_path(path: &str) -> Result<String, IndexError> {
	let mut resolved = Vec::new();
	for component in Path::new(path).components() {
		match component {
			Component::Normal(component) => resolved.push(component.to_string_lossy().into_owned()),
			Component::ParentDir => {
				if resolved.pop().is_none() {
					return Err(IndexError::InvalidPath {
						path: path.to_owned(),
						reason: "path escapes the mount root".to_owned(),
					});
				}
			}
			Component::CurDir => {}
			Component::RootDir | Component::Prefix(_) => {
				return Err(IndexError::InvalidPath {
					path: path.to_owned(),
					reason: "path is not relative".to_owned(),
				});
			}
		}
	}

	if resolved.is_empty() {
		return Err(IndexError::InvalidPath {
			path: path.to_owned(),
			reason: "path has no file name".to_owned(),
		});
	}
	Ok(resolved.join("/"))
}

#[cfg(test)]
mod tests {
	use backbeat_core::{AssetPath, BackbeatFile, ChartData, ChartDesc};
	use backbeat_store_config::BackbeatConfig;
	use tempfile::TempDir;

	use crate::NodeKind;

	use super::*;

	fn asset_path(value: &str) -> AssetPath {
		AssetPath::from_path(value).unwrap()
	}

	fn candidate(
		filename: &str,
		artist: Option<&str>,
		title: Option<&str>,
		assets: &Assets,
		memberships: Vec<PackMembership>,
	) -> StepmaniaChartCandidate {
		StepmaniaChartCandidate {
			chart: StoredStepmaniaChart {
				bundle_id: BundleId(Sha256::null()),
				filename: ChartFilename::from_path(filename).expect("filename"),
				sha256: Sha256::null(),
				size: 0,
			},
			combined_assets_id: CombinedAssetsId::compute(assets),
			description: match (artist, title) {
				(Some(artist), Some(title)) => format!("{artist} - {title}"),
				(Some(artist), None) => artist.to_owned(),
				(None, Some(title)) => title.to_owned(),
				(None, None) => "Unknown Title".to_owned(),
			},
			memberships,
		}
	}

	#[test]
	fn folders_use_pack_tags_at_the_mount_root() {
		let assets = Assets::new();
		let folders = folder_chart_candidates(vec![
			candidate(
				"chart.sm",
				Some("Artist"),
				Some("Title"),
				&assets,
				vec![PackMembership {
					pack_name: "Pack".to_owned(),
					song_folder: Some("Original Folder".to_owned()),
				}],
			),
			candidate(
				"chart.ssc",
				Some("Artist"),
				Some("Title"),
				&assets,
				Vec::new(),
			),
		]);

		assert!(
			folders
				.iter()
				.any(|folder| { folder.parent == "Pack" && folder.name == "Original Folder" })
		);
		assert!(folders.iter().any(|folder| {
			folder.parent == "_Backbeat Other" && folder.name == "Artist - Title"
		}));
	}

	#[test]
	fn folders_allow_assets_one_level_above_the_song() {
		let asset_id = AssetId::from(Sha256::checksum_bytes(b"asset"));
		let assets = HashMap::from([(asset_path("../shared.wav"), asset_id)]);
		let chart = candidate(
			"chart.sm",
			Some("Artist"),
			Some("Title"),
			&assets,
			Vec::new(),
		);
		let combined_assets_id = chart.combined_assets_id;
		let index = build_index_from_folders(
			folder_chart_candidates(vec![chart]),
			HashMap::from([(
				combined_assets_id,
				AssetSet {
					assets,
					sizes: HashMap::from([(asset_id, 1)]),
				},
			)]),
			Vec::new(),
		)
		.expect("build index");

		assert!(
			index
				.lookup_path("_Backbeat Other/Artist - Title/chart.sm")
				.is_some()
		);
		assert!(index.lookup_path("_Backbeat Other/shared.wav").is_some());
	}

	#[test]
	fn asset_conflicts_at_the_pack_level_choose_the_lowest_hash() {
		let first = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second = AssetId::from(Sha256::checksum_bytes(b"second"));
		let first_assets = HashMap::from([(asset_path("../shared.png"), first)]);
		let second_assets = HashMap::from([(asset_path("../shared.png"), second)]);
		let first_id = CombinedAssetsId::compute(&first_assets);
		let second_id = CombinedAssetsId::compute(&second_assets);
		let index = build_index_from_folders(
			vec![
				StepmaniaFolder {
					parent: "Pack".to_owned(),
					base_name: "First".to_owned(),
					name: "First".to_owned(),
					combined_assets_id: first_id,
					charts: Vec::new(),
				},
				StepmaniaFolder {
					parent: "Pack".to_owned(),
					base_name: "Second".to_owned(),
					name: "Second".to_owned(),
					combined_assets_id: second_id,
					charts: Vec::new(),
				},
			],
			HashMap::from([
				(
					first_id,
					AssetSet {
						assets: first_assets,
						sizes: HashMap::from([(first, 1)]),
					},
				),
				(
					second_id,
					AssetSet {
						assets: second_assets,
						sizes: HashMap::from([(second, 1)]),
					},
				),
			]),
			Vec::new(),
		)
		.expect("build index");

		let asset_inode = index.lookup_path("Pack/shared.png").expect("asset path");
		let NodeKind::Asset { asset_id, .. } = &index.get(asset_inode).expect("asset node").kind
		else {
			panic!("expected asset");
		};
		assert_eq!(*asset_id, if first.0 < second.0 { first } else { second });
	}

	#[test]
	fn pack_assets_are_projected_at_the_pack_root() {
		let asset_id = AssetId::from(Sha256::checksum_bytes(b"pack asset"));
		let index = build_index_from_folders(
			Vec::new(),
			HashMap::new(),
			vec![PackAsset {
				pack_name: "Pack".to_owned(),
				path: "banner.png".to_owned(),
				asset_id,
				size: 1,
			}],
		)
		.expect("build index");

		assert!(index.lookup_path("Pack/banner.png").is_some());
	}

	#[test]
	fn colliding_pack_assets_choose_the_lowest_hash() {
		let first = AssetId::from(Sha256::checksum_bytes(b"first"));
		let second = AssetId::from(Sha256::checksum_bytes(b"second"));
		let index = build_index_from_folders(
			Vec::new(),
			HashMap::new(),
			vec![
				PackAsset {
					pack_name: "Pack".to_owned(),
					path: "banner.png".to_owned(),
					asset_id: first,
					size: 1,
				},
				PackAsset {
					pack_name: "Pack".to_owned(),
					path: "banner.png".to_owned(),
					asset_id: second,
					size: 1,
				},
			],
		)
		.expect("build index");

		let asset_inode = index.lookup_path("Pack/banner.png").expect("asset path");
		let NodeKind::Asset { asset_id, .. } = &index.get(asset_inode).expect("asset node").kind
		else {
			panic!("expected asset");
		};
		assert_eq!(*asset_id, if first.0 < second.0 { first } else { second });
	}

	#[test]
	fn duplicate_chart_paths_become_a_meld_recipe_without_deduplicating_sources() {
		let assets = Assets::new();
		let mut first = candidate(
			"song.sm",
			Some("Artist"),
			Some("Title"),
			&assets,
			Vec::new(),
		);
		first.chart.bundle_id = BundleId(Sha256::checksum_bytes(b"first bundle"));
		let mut second = first.clone();
		second.chart.bundle_id = BundleId(Sha256::checksum_bytes(b"second bundle"));

		let combined_assets_id = first.combined_assets_id;
		let index = build_index_from_folders(
			folder_chart_candidates(vec![first.clone(), second.clone()]),
			HashMap::from([(combined_assets_id, AssetSet::default())]),
			Vec::new(),
		)
		.expect("build index");

		let inode = index
			.lookup_path("_Backbeat Other/Artist - Title/song.sm")
			.expect("melded chart path");
		let NodeKind::MeldedChart { bundle_ids } = &index.get(inode).expect("melded chart").kind
		else {
			panic!("expected melded chart");
		};
		assert_eq!(
			bundle_ids.as_ref(),
			&[first.chart.bundle_id, second.chart.bundle_id]
		);
	}

	#[test]
	fn duplicate_chart_paths_require_a_meldable_format() {
		let assets = Assets::new();
		let first = candidate(
			"song.txt",
			Some("Artist"),
			Some("Title"),
			&assets,
			Vec::new(),
		);
		let second = first.clone();
		let combined_assets_id = first.combined_assets_id;

		let error = build_index_from_folders(
			folder_chart_candidates(vec![first, second]),
			HashMap::from([(combined_assets_id, AssetSet::default())]),
			Vec::new(),
		)
		.unwrap_err();
		assert!(matches!(
			error,
			IndexError::UnmeldableChartCollision { filename, .. } if filename == "song.txt"
		));
	}

	#[test]
	fn filesystem_reads_duplicate_sm_charts_as_one_melded_file() {
		let temp = TempDir::new().expect("create temp directory");
		let config_dir = temp.path().join("config");
		let mut config = BackbeatConfig::default();
		config.store.path = temp.path().join("store");
		config
			.write_to_dir(&config_dir)
			.expect("write configuration");
		let store = Backbeat::open_with_overridden_config_dir(&config_dir).expect("open store");

		for notes in [
			b"#TITLE:Song;\n#ARTIST:Artist;\n#NOTES:dance-single:Author:Easy:3:0:0000;\n"
				.as_slice(),
			b"#TITLE:Song;\n#ARTIST:Artist;\n#NOTES:dance-single:Author:Hard:9:0:0000;\n"
				.as_slice(),
		] {
			store
				.import_bundle(&BackbeatFile {
					filename: ChartFilename::from_path("song.sm").unwrap(),
					assets: Assets::new(),
					desc: ChartDesc::new("test").unwrap(),
					chart: ChartData::compress(notes).unwrap(),
				})
				.expect("import fractured chart");
		}

		let runtime = tokio::runtime::Runtime::new().expect("create runtime");
		let filesystem = runtime
			.block_on(build_filesystem(store))
			.expect("build filesystem");
		let other_folder = filesystem
			.index()
			.lookup_child(crate::ROOT_INODE, "_Backbeat Other")
			.expect("other folder");
		let song_folder = filesystem
			.readdir_entries(other_folder, 0)
			.expect("list song folders")
			.into_iter()
			.find(|entry| entry.name != "." && entry.name != "..")
			.expect("song folder")
			.ino;
		let inode = filesystem
			.index()
			.lookup_child(song_folder, "song.sm")
			.expect("melded chart");

		let attrs = filesystem.getattr(inode).expect("get attributes");
		let handle = filesystem.open(inode).expect("open melded chart");
		let bytes = filesystem
			.read(handle, 0, u32::MAX)
			.expect("read melded chart");
		assert_eq!(attrs.size, bytes.len() as u64);
		assert_eq!(
			rg_formats::sm::from_bytes(&bytes, "song.sm")
				.expect("parse melded chart")
				.len(),
			2
		);
	}
}
