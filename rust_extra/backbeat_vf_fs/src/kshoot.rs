use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use backbeat_core::collections::PackBundleTags;
use backbeat_core::{AssetId, ChartFilename, Sha256};
use backbeat_core::{Assets, CombinedAssetsId};
use backbeat_sdk::Backbeat;
use futures::TryStreamExt;

use crate::database::open_readonly_database;
use crate::jails;
use crate::safe_paths::{MAX_FOLDER_NAME_CHARS, sanitize_folder_name, truncate_folder_name};
use crate::{BackbeatFilesystem, IndexError, MemDentryIndex, PathIndex};

#[derive(Debug, Clone)]
struct StoredKshootChart {
	filename: ChartFilename,
	sha256: Sha256,
	size: u64,
}

#[derive(Debug, Clone)]
struct PackMembership {
	pack_folder: String,
	song_folder: Option<String>,
}

#[derive(Debug, Clone)]
struct KshootChartCandidate {
	chart: StoredKshootChart,
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
	pack_folder: String,
	path: String,
	asset_id: AssetId,
	size: u64,
}

#[derive(Debug, Default)]
struct KshootCharts {
	candidates: Vec<KshootChartCandidate>,
	assets: HashMap<CombinedAssetsId, AssetSet>,
	pack_assets: Vec<PackAsset>,
}

#[derive(Debug, Clone)]
struct KshootFolder {
	parent: String,
	base_name: String,
	name: String,
	combined_assets_id: CombinedAssetsId,
	charts: Vec<StoredKshootChart>,
}

/// Build the backend-neutral filesystem for the K-Shoot virtual-folder prefab.
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

async fn load_chart_candidates(store: &Backbeat) -> Result<KshootCharts, IndexError> {
	let pool = open_readonly_database(store).await?;
	let mut charts = KshootCharts::default();
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
		WHERE b.extension IN ('ksh', 'kson')
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
		charts.candidates.push(KshootChartCandidate {
			chart: StoredKshootChart {
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

	let pack_folders = load_pack_folders(&pool).await?;
	let mut membership_rows = sqlx::query!(
		r#"
		SELECT
			pe.bundle_id AS "bundle_id: backbeat_core::BundleId",
			p.url AS pack_url,
			pe.tags AS tags
		FROM pack_entry pe
		JOIN pack p ON p.url = pe.url
		JOIN bundle b ON b.id = pe.bundle_id
		WHERE b.extension IN ('ksh', 'kson')
		ORDER BY p.url, pe.entry
		"#,
	)
	.fetch(&pool);
	while let Some(row) = membership_rows.try_next().await? {
		let Some(&candidate_index) = candidate_indexes.get(&row.bundle_id) else {
			continue;
		};
		let pack_folder = pack_folders
			.get(&row.pack_url)
			.expect("K-Shoot pack entry must belong to a loaded K-Shoot pack")
			.clone();
		let tags: PackBundleTags =
			serde_json::from_str(&row.tags).map_err(|_| IndexError::InvalidValue {
				field: "pack entry tags",
				value: row.tags,
			})?;
		let song_folder = tags
			.get("kshoot/song-folder")
			.filter(|name| !name.trim().is_empty())
			.cloned();
		charts.candidates[candidate_index]
			.memberships
			.push(PackMembership {
				pack_folder,
				song_folder,
			});
	}
	drop(membership_rows);

	let mut asset_rows = sqlx::query!(
		r#"
		WITH representatives AS (
			SELECT MIN(id) AS bundle_id
			FROM bundle
			WHERE extension IN ('ksh', 'kson')
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
		let combined_assets_id = representative_ids
			.get(&row.bundle_id)
			.expect("K-Shoot asset row must belong to a loaded K-Shoot bundle");
		let assets = charts
			.assets
			.get_mut(combined_assets_id)
			.expect("K-Shoot asset set must exist");
		assets.assets.insert(row.path, row.asset_id);
		assets.sizes.insert(row.asset_id, row.size.max(0) as u64);
	}
	drop(asset_rows);

	let mut pack_asset_rows = sqlx::query!(
		r#"
		SELECT
			p.url AS pack_url,
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
			  AND b.extension IN ('ksh', 'kson')
		)
		"#,
	)
	.fetch(&pool);
	while let Some(row) = pack_asset_rows.try_next().await? {
		let pack_folder = pack_folders
			.get(&row.pack_url)
			.expect("K-Shoot pack asset must belong to a loaded K-Shoot pack")
			.clone();
		charts.pack_assets.push(PackAsset {
			pack_folder,
			path: row.path,
			asset_id: row.asset_id,
			size: row.size.max(0) as u64,
		});
	}

	Ok(charts)
}

async fn load_pack_folders(pool: &sqlx::SqlitePool) -> Result<HashMap<String, String>, IndexError> {
	let rows = sqlx::query!(
		r#"
		SELECT p.url, p.name
		FROM pack p
		WHERE EXISTS (
			SELECT 1
			FROM pack_entry pe
			JOIN bundle b ON b.id = pe.bundle_id
			WHERE pe.url = p.url
			  AND b.extension IN ('ksh', 'kson')
		)
		"#
	)
	.fetch_all(pool)
	.await?;
	Ok(pack_folder_names(
		rows.into_iter().map(|row| (row.url, row.name)),
	))
}

fn folder_chart_candidates(candidates: Vec<KshootChartCandidate>) -> Vec<KshootFolder> {
	let metadata = consensus_metadata(&candidates);
	let mut folders = HashMap::<(String, String, CombinedAssetsId), KshootFolder>::new();

	for candidate in candidates {
		let fallback_name = metadata
			.get(&candidate.combined_assets_id)
			.expect("K-Shoot chart must have consensus metadata");
		let placements = if candidate.memberships.is_empty() {
			vec![("Other Charts".to_owned(), fallback_name.clone())]
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
					(membership.pack_folder.clone(), name)
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
				.or_insert_with(|| KshootFolder {
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

fn consensus_metadata(candidates: &[KshootChartCandidate]) -> HashMap<CombinedAssetsId, String> {
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

fn pack_folder_names(packs: impl IntoIterator<Item = (String, String)>) -> HashMap<String, String> {
	let mut packs_by_name = BTreeMap::<String, Vec<String>>::new();
	for (url, name) in packs {
		packs_by_name
			.entry(pack_folder_name(&name))
			.or_default()
			.push(url);
	}

	let mut folders = HashMap::new();
	for (name, urls) in packs_by_name {
		let duplicate = urls.len() > 1;
		for url in urls {
			let folder = if duplicate {
				let suffix = format!(
					" ({})",
					&Sha256::checksum_bytes(url.as_bytes()).to_string()[..6]
				);
				format!(
					"{}{suffix}",
					truncate_folder_name(&name, MAX_FOLDER_NAME_CHARS - suffix.chars().count())
				)
			} else {
				name.clone()
			};
			folders.insert(url, folder);
		}
	}
	folders
}

fn song_folder_name(name: &str) -> String {
	truncate_folder_name(
		&sanitize_folder_name(name, "Unknown Title"),
		MAX_FOLDER_NAME_CHARS,
	)
}

fn build_index_from_folders(
	folders: Vec<KshootFolder>,
	assets_by_id: HashMap<CombinedAssetsId, AssetSet>,
	pack_assets: Vec<PackAsset>,
) -> Result<PathIndex, IndexError> {
	let mut index = PathIndex::new();
	for folder in folders {
		let assets = assets_by_id
			.get(&folder.combined_assets_id)
			.expect("K-Shoot folder must have an asset set");
		let jail = jails::path(jails::depth(&assets.assets));
		let folder_path = format!("{}/{}/{}", folder.parent, folder.name, jail);
		for chart in &folder.charts {
			index.insert_chart(
				&format!("{folder_path}{}", chart.filename),
				chart.sha256,
				chart.size,
			)?;
		}
		for (asset_path, asset_id) in &assets.assets {
			let size = assets.sizes[asset_id];
			index.insert_asset(&format!("{folder_path}{asset_path}"), *asset_id, size)?;
		}
	}
	for asset in pack_assets {
		index.insert_asset(
			&format!("{}/{path}", asset.pack_folder, path = asset.path),
			asset.asset_id,
			asset.size,
		)?;
	}
	Ok(index)
}

#[cfg(test)]
mod tests {
	use backbeat_core::AssetPath;

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
	) -> KshootChartCandidate {
		KshootChartCandidate {
			chart: StoredKshootChart {
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
	fn folders_use_pack_tags_then_artist_and_title() {
		let assets = Assets::new();
		let folders = folder_chart_candidates(vec![
			candidate(
				"adv.ksh",
				Some("Artist"),
				Some("Title"),
				&assets,
				vec![PackMembership {
					pack_folder: "Pack".to_owned(),
					song_folder: Some("Original Folder".to_owned()),
				}],
			),
			candidate(
				"exh.ksh",
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
		assert!(
			folders.iter().any(|folder| {
				folder.parent == "Other Charts" && folder.name == "Artist - Title"
			})
		);
	}

	#[test]
	fn folders_disambiguate_sanitised_song_names() {
		let first = HashMap::from([(
			asset_path("sound.wav"),
			AssetId::from(Sha256::checksum_bytes(b"first")),
		)]);
		let second = HashMap::from([(
			asset_path("sound.wav"),
			AssetId::from(Sha256::checksum_bytes(b"second")),
		)]);
		let folders = folder_chart_candidates(vec![
			candidate(
				"a.ksh",
				Some("Artist"),
				Some("Song/Name"),
				&first,
				Vec::new(),
			),
			candidate(
				"b.ksh",
				Some("Artist"),
				Some("Song:Name"),
				&second,
				Vec::new(),
			),
		]);

		assert_eq!(folders.len(), 2);
		assert!(
			folders
				.iter()
				.all(|folder| folder.name.starts_with("Artist - Song-Name ("))
		);
		assert_ne!(folders[0].name, folders[1].name);
	}

	#[test]
	fn folders_jail_assets_for_up_references() {
		let asset_id = AssetId::from(Sha256::checksum_bytes(b"asset"));
		let assets = HashMap::from([(asset_path("../shared.wav"), asset_id)]);
		let chart = candidate(
			"adv.ksh",
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
				.lookup_path("Other Charts/Artist - Title/_x/adv.ksh")
				.is_some()
		);
		assert!(
			index
				.lookup_path("Other Charts/Artist - Title/shared.wav")
				.is_some()
		);
	}

	#[test]
	fn pack_assets_are_projected_below_the_pack_folder() {
		let asset_id = AssetId::from(Sha256::checksum_bytes(b"pack asset"));
		let index = build_index_from_folders(
			Vec::new(),
			HashMap::new(),
			vec![PackAsset {
				pack_folder: "Pack".to_owned(),
				path: "banner.png".to_owned(),
				asset_id,
				size: 1,
			}],
		)
		.expect("build index");

		assert!(index.lookup_path("Pack/banner.png").is_some());
	}

	#[test]
	fn duplicate_pack_names_get_url_hash_suffixes() {
		let assets = Assets::new();
		let folders = pack_folder_names([
			(
				"https://example.test/one".to_owned(),
				"Duplicate Pack".to_owned(),
			),
			(
				"https://example.test/two".to_owned(),
				"Duplicate Pack".to_owned(),
			),
		]);
		let first_folder = folders["https://example.test/one"].clone();
		let second_folder = folders["https://example.test/two"].clone();
		assert_ne!(first_folder, second_folder);
		assert!(first_folder.starts_with("Duplicate Pack ("));
		assert!(second_folder.starts_with("Duplicate Pack ("));

		let first = candidate(
			"chart.ksh",
			Some("Artist"),
			Some("Title"),
			&assets,
			vec![PackMembership {
				pack_folder: first_folder.clone(),
				song_folder: Some("Song".to_owned()),
			}],
		);
		let mut second = first.clone();
		second.chart.sha256 = Sha256::checksum_bytes(b"second chart");
		second.memberships[0].pack_folder = second_folder.clone();

		let index = build_index_from_folders(
			folder_chart_candidates(vec![first, second]),
			HashMap::from([(CombinedAssetsId::compute(&assets), AssetSet::default())]),
			Vec::new(),
		)
		.expect("build index");
		assert!(
			index
				.lookup_path(&format!("{first_folder}/Song/chart.ksh"))
				.is_some()
		);
		assert!(
			index
				.lookup_path(&format!("{second_folder}/Song/chart.ksh"))
				.is_some()
		);
	}
}
