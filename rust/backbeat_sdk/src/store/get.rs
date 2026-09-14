//! Retrieve pieces of data from the backbeat store.

use backbeat_core::ChartDesc;
use backbeat_core::ChartFilename;
use backbeat_core::CollectionKind;
use backbeat_core::{Assets, BackbeatFile, BundleId, ChartData};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::Result;
use crate::store::Backbeat;
use crate::util::BLOCK;

pub(crate) struct SomeChartInfo {
	pub(crate) filename: ChartFilename,
	pub(crate) desc: ChartDesc,
	pub(crate) bundle_id: BundleId,
	pub(crate) data: Vec<u8>,
}

pub(crate) fn assemble_bundle(
	store: &Backbeat,
	SomeChartInfo {
		filename,
		desc,
		bundle_id,
		data,
	}: SomeChartInfo,
) -> Result<BackbeatFile> {
	let asset_rows = BLOCK(
		sqlx::query!(
			r#"
			SELECT
				path AS "path: backbeat_core::AssetPath",
				sha256 AS "asset_id!: backbeat_core::AssetId"
			 FROM
				asset_map
			WHERE
				combined_assets_id = (SELECT combined_assets_id FROM bundle WHERE id = ?1)
			"#,
			bundle_id,
		)
		.fetch_all(&store.pool),
	)?;

	let mut assets: Assets = HashMap::new();
	for row in asset_rows {
		assets.insert(row.path, row.asset_id);
	}

	Ok(BackbeatFile {
		filename,
		assets,
		desc,
		chart: ChartData::from_compressed(data)?,
	})
}

pub(crate) fn load_collection_assets(
	pool: &SqlitePool,
	kind: CollectionKind,
	url: &str,
) -> Result<Assets> {
	let mut assets = Assets::new();
	match kind {
		CollectionKind::Table => {
			for row in BLOCK(
				sqlx::query!(
					r#"
					SELECT
						path AS "path: backbeat_core::AssetPath",
						sha256 AS "sha256: backbeat_core::AssetId"
					FROM
						difftable_asset
					WHERE
						url = ?1
					"#,
					url
				)
				.fetch_all(pool),
			)? {
				assets.insert(row.path, row.sha256);
			}
		}
		CollectionKind::Course => {
			for row in BLOCK(
				sqlx::query!(
					r#"
					SELECT
						path AS "path: backbeat_core::AssetPath",
						sha256 AS "sha256: backbeat_core::AssetId"
					FROM
						course_asset
					WHERE
						url = ?1
					"#,
					url
				)
				.fetch_all(pool),
			)? {
				assets.insert(row.path, row.sha256);
			}
		}
		CollectionKind::Pack => {
			for row in BLOCK(
				sqlx::query!(
					r#"
				SELECT
					path AS "path: backbeat_core::AssetPath",
					sha256 AS "sha256: backbeat_core::AssetId"
				FROM
					pack_asset
				WHERE
					url = ?1
				"#,
					url
				)
				.fetch_all(pool),
			)? {
				assets.insert(row.path, row.sha256);
			}
		}
	}
	Ok(assets)
}

#[cfg(test)]
mod tests {
	use backbeat_core::{ChartFilename, ChartId, IdAlgorithm, Sha256};
	use backbeat_store_config::BackbeatConfig;
	use tempfile::TempDir;

	use super::*;

	#[test]
	fn sha256_chart_ids_query_text_columns() {
		let temp = TempDir::new().unwrap();
		let config_dir = temp.path().join("config");
		let mut config = BackbeatConfig::default();
		config.store.path = temp.path().join("store");
		config.write_to_dir(&config_dir).unwrap();
		let store = Backbeat::open_with_overridden_config_dir(&config_dir).unwrap();

		let raw = b"#TITLE:Test;\n#BPMS:0=120;\n#NOTES:dance-single:A:Hard:9:0:0000;\n";
		let bundle = BackbeatFile {
			filename: ChartFilename::from_path("test.sm").unwrap(),
			desc: ChartDesc::new("test").unwrap(),
			assets: Assets::new(),
			chart: ChartData::compress(raw).unwrap(),
		};
		store.import_bundle(&bundle).unwrap();

		let chart_id = ChartId {
			alg: IdAlgorithm::Sha256,
			val: Sha256::checksum_bytes(raw).to_string(),
		};

		assert!(store.has_chart(&chart_id).unwrap());
		assert_eq!(store.get_chart_data(&chart_id).unwrap(), raw);
		assert_eq!(
			store.get_chart(&chart_id).unwrap().filename,
			bundle.filename
		);
	}
}
