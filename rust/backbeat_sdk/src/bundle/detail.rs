use backbeat_core::AssetId;
use backbeat_core::AssetPath;
use backbeat_core::BundleId;
use backbeat_core::ChartFilename;
use backbeat_core::ChartId;
use backbeat_core::CollectionKind;
use backbeat_core::Sha256;
use serde::Deserialize;
use serde::Serialize;

use crate::Result;
use crate::store::Backbeat;
use crate::util::BLOCK;

/// In what locally installed collections does this bundle appear?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCollectionAppearance {
	pub url: String,
	pub collection_kind: CollectionKind,
	pub name: String,
	pub symbol: Option<String>,
	pub level: Option<String>,
}

/// One asset a bundle depends on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAssetDep {
	pub path: AssetPath,
	pub asset_id: AssetId,
	/// Size in bytes, if the asset has actually been stored. `None` means
	/// this bundle references an asset that is not downloaded.
	pub size: Option<u64>,
}

/// Full detail for a single bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDetail {
	pub bundle_id: BundleId,
	pub filename: ChartFilename,
	pub description: String,
	pub chart_sha256: Sha256,
	pub uncompressed_size: u64,
	pub chart_ids: Vec<ChartId>,
	pub assets: Vec<BundleAssetDep>,
	pub appearances: Vec<BundleCollectionAppearance>,
}

pub(crate) fn collection_appearances(
	store: &Backbeat,
	bundle_id: BundleId,
	chart_sha256: Sha256,
) -> Result<Vec<BundleCollectionAppearance>> {
	let mut appearances = Vec::new();

	let pack_rows = BLOCK(
		sqlx::query!(
			r#"
				SELECT
					pack.url,
					pack.name
				FROM
					pack
				JOIN
					pack_entry ON pack_entry.url = pack.url
				WHERE
					pack_entry.bundle_id = ?1
				ORDER BY
					pack.name,
					pack.url
				"#,
			bundle_id,
		)
		.fetch_all(&store.pool),
	)?;
	appearances.extend(pack_rows.into_iter().map(|row| BundleCollectionAppearance {
		url: row.url,
		collection_kind: CollectionKind::Pack,
		name: row.name,
		symbol: None,
		level: None,
	}));

	let table_rows = BLOCK(
		sqlx::query!(
			r#"
				SELECT
					difftable.url,
					difftable.name,
					difftable.symbol,
					difftable_chart.level
				FROM
					difftable
				JOIN
					difftable_chart ON difftable_chart.url = difftable.url
				WHERE
					(difftable_chart.id = 'sha256/' || ?1)
					OR EXISTS (
						SELECT
							1
						FROM
							chart_id
						WHERE
							chart_id.chart_sha256 = ?2
							AND chart_id.id = difftable_chart.id
					)
				GROUP BY
					difftable.url
				ORDER BY
					difftable.name,
					difftable.url
				"#,
			chart_sha256,
			chart_sha256,
		)
		.fetch_all(&store.pool),
	)?;
	appearances.extend(
		table_rows
			.into_iter()
			.map(|row| BundleCollectionAppearance {
				url: row.url,
				collection_kind: CollectionKind::Table,
				name: row.name,
				symbol: Some(row.symbol),
				level: Some(row.level),
			}),
	);

	let course_rows = BLOCK(
		sqlx::query!(
			r#"
				SELECT DISTINCT
					course.url,
					course.name
				FROM
					course
				JOIN
					course_chart ON course_chart.url = course.url
				WHERE
					(course_chart.id = 'sha256/' || ?1)
					OR EXISTS (
						SELECT
							1
						FROM
							chart_id
						WHERE
							chart_id.chart_sha256 = ?2
							AND chart_id.id = course_chart.id
					)
				ORDER BY
					course.name,
					course.url
				"#,
			chart_sha256,
			chart_sha256,
		)
		.fetch_all(&store.pool),
	)?;
	appearances.extend(
		course_rows
			.into_iter()
			.map(|row| BundleCollectionAppearance {
				url: row.url,
				collection_kind: CollectionKind::Course,
				name: row.name,
				symbol: None,
				level: None,
			}),
	);

	appearances.sort_by(|a, b| {
		a.name
			.cmp(&b.name)
			.then_with(|| a.url.cmp(&b.url))
			.then_with(|| a.collection_kind.as_str().cmp(b.collection_kind.as_str()))
	});
	Ok(appearances)
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::AssetPath;
	use backbeat_core::ValidGamemodeIdentifier;
	use backbeat_core::asset_id::AssetId;
	use backbeat_core::collections::{
		Course, CourseChart, Pack, PackBundle, Table, TableChart, TableLevel,
	};
	use backbeat_core::{BackbeatFile, ChartData, ChartDesc};
	use chrono::Utc;

	use super::*;
	use crate::StoreError;

	#[test]
	fn chart_detail_returns_not_found_for_missing_bundle() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_chart_detail_missing");

		let bundle_id: BundleId = format!("b-{}", "0".repeat(64)).parse().unwrap();
		assert!(matches!(
			store.bundle_detail(bundle_id),
			Err(StoreError::NotFound(_))
		));
	}

	#[test]
	fn chart_detail_reports_dependencies_and_missing_asset() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_chart_detail");

		let stored_data = b"actually stored";
		let stored_asset = AssetId(Sha256::checksum_bytes(stored_data));
		crate::test_support::store_asset_unchecked(&store, stored_asset, stored_data)
			.expect("store_asset");

		let missing_asset = AssetId(Sha256::checksum_bytes(b"never stored"));
		let stored_path = AssetPath::from_path("sound/kick.ogg").unwrap();
		let missing_path = AssetPath::from_path("sound/missing.ogg").unwrap();

		let mut assets = HashMap::new();
		assets.insert(stored_path.clone(), stored_asset);
		assets.insert(missing_path.clone(), missing_asset);

		let bb = BackbeatFile {
			filename: ChartFilename::from_path("song.bms").unwrap(),
			assets,
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#ARTIST Camellia\n#TITLE Songtitle\n").unwrap(),
		};
		let bundle_id = store.import_bundle(&bb).expect("import_bb_with_report");

		let detail = store.bundle_detail(bundle_id).expect("bundle should exist");

		assert_eq!(
			detail.filename,
			ChartFilename::from_path("song.bms").unwrap()
		);
		assert_eq!(detail.description, "test");
		assert_eq!(detail.assets.len(), 2);

		let by_path: HashMap<_, _> = detail.assets.iter().map(|d| (d.path.clone(), d)).collect();
		assert_eq!(by_path[&stored_path].size, Some(stored_data.len() as u64));
		assert_eq!(by_path[&missing_path].size, None);

		let wire = serde_json::to_value(&detail).expect("serialize bundle detail");
		assert!(wire.get("gamemode").is_none());
		assert!(wire["assets"][0].get("asset_id").is_some());
		assert!(wire["assets"][0].get("sha256").is_none());
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn bundle_detail_reports_collection_appearances() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_bundle_detail_appearances");
		let bb = BackbeatFile {
			filename: ChartFilename::from_path("song.bms").unwrap(),
			assets: HashMap::new(),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#TITLE Songtitle\n").unwrap(),
		};
		let bundle_id = store.import_bundle(&bb).expect("import_bb");
		let initial_detail = store.bundle_detail(bundle_id).expect("bundle should exist");
		let sha256_id: ChartId = format!("sha256/{}", initial_detail.chart_sha256)
			.parse()
			.expect("sha256 chart id");

		store
			.pack_put(
				"https://example.test/pack",
				&Pack {
					name: "Pack".to_owned(),
					gamemode: ValidGamemodeIdentifier::new("bms-5k").unwrap(),
					updated: Utc::now(),
					tags: Default::default(),
					assets: Default::default(),
					bundles: vec![PackBundle {
						id: bundle_id,
						desc: "Song".to_owned(),
						tags: Default::default(),
					}],
				},
			)
			.await
			.expect("store pack");
		store
			.table_put(
				"https://example.test/table",
				&Table {
					name: "Table".to_owned(),
					symbol: "L".to_owned(),
					gamemode: ValidGamemodeIdentifier::new("bms-5k").unwrap(),
					updated: Utc::now(),
					tags: Default::default(),
					assets: Default::default(),
					levels: vec![
						TableLevel {
							level: "1".to_owned(),
							tags: Default::default(),
							charts: vec![TableChart {
								id: sha256_id.clone(),
								desc: "Song".to_owned(),
								tags: Default::default(),
							}],
						},
						TableLevel {
							level: "2".to_owned(),
							tags: Default::default(),
							charts: vec![TableChart {
								id: sha256_id.clone(),
								desc: "Song again".to_owned(),
								tags: Default::default(),
							}],
						},
					],
					folders: Vec::new(),
				},
			)
			.await
			.expect("store table");
		store
			.course_put(
				"https://example.test/course",
				&Course {
					name: "Course".to_owned(),
					updated: Utc::now(),
					gamemode: ValidGamemodeIdentifier::new("bms-5k").unwrap(),
					tags: Default::default(),
					assets: Default::default(),
					charts: vec![CourseChart {
						id: sha256_id,
						desc: "Song".to_owned(),
						tags: Default::default(),
					}],
				},
			)
			.await
			.expect("store course");

		let detail = store.bundle_detail(bundle_id).expect("bundle should exist");
		assert_eq!(
			detail.appearances,
			vec![
				BundleCollectionAppearance {
					url: "https://example.test/course".to_owned(),
					collection_kind: CollectionKind::Course,
					name: "Course".to_owned(),
					symbol: None,
					level: None,
				},
				BundleCollectionAppearance {
					url: "https://example.test/pack".to_owned(),
					collection_kind: CollectionKind::Pack,
					name: "Pack".to_owned(),
					symbol: None,
					level: None,
				},
				BundleCollectionAppearance {
					url: "https://example.test/table".to_owned(),
					collection_kind: CollectionKind::Table,
					name: "Table".to_owned(),
					symbol: Some("L".to_owned()),
					level: Some("1".to_owned()),
				},
			]
		);

		let wire = serde_json::to_value(&detail).expect("serialize bundle detail");
		assert_eq!(wire["appearances"][0]["collection_kind"], "course");
		assert!(wire["appearances"][0].get("collectionKind").is_none());
		assert_eq!(wire["appearances"][2]["symbol"], "L");
		assert_eq!(wire["appearances"][2]["level"], "1");

		store
			.collection_rm("https://example.test/pack", true)
			.unwrap();
		assert!(store.has_bundle(bundle_id).unwrap());
		store
			.collection_rm("https://example.test/table", true)
			.unwrap();
		assert!(store.has_bundle(bundle_id).unwrap());
		store
			.collection_rm("https://example.test/course", true)
			.unwrap();
		assert!(!store.has_bundle(bundle_id).unwrap());
	}
}
