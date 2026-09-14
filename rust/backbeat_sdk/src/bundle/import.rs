#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{AssetId, AssetPath, ChartDesc, ChartFilename, Sha256};
	use backbeat_core::{BackbeatFile, ChartData};

	use crate::util::BLOCK;

	#[test]
	fn importing_bundle_persists_combined_assets_id() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_import_extraction");
		let asset_id = AssetId::from(Sha256::checksum_bytes(b"asset"));
		let bb = BackbeatFile {
			filename: ChartFilename::from_path("song.bms").unwrap(),
			assets: HashMap::from([(AssetPath::from_path("song.ogg").unwrap(), asset_id)]),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#TITLE Song\n#ARTIST Artist\n").unwrap(),
		};

		let bundle_id = store.import_bundle(&bb).expect("import_bb");
		let row = BLOCK(async {
			sqlx::query!(
				r#"
					SELECT
						COALESCE(combined_assets_id, '') AS "combined_assets_id!: String"
					FROM bundle
					WHERE id = ?1
				"#,
				bundle_id,
			)
			.fetch_one(&store.pool)
			.await
		})
		.expect("load persisted bundle");

		assert_eq!(row.combined_assets_id, bb.combined_assets_id().to_string());

		let hash_storage = BLOCK(async {
			sqlx::query!(
				r#"
					SELECT
						typeof(cd.sha256) AS "chart_type!: String",
						length(cd.sha256) AS "chart_length!: i64",
						typeof(am.sha256) AS "asset_type!: String",
						length(am.sha256) AS "asset_length!: i64"
					FROM bundle b
					JOIN chart_data cd ON cd.sha256 = b.chart_sha256
					JOIN asset_map am ON am.combined_assets_id = b.combined_assets_id
					WHERE b.id = ?1
				"#,
				bundle_id,
			)
			.fetch_one(&store.pool)
			.await
		})
		.expect("load hash storage");
		assert_eq!(hash_storage.chart_type, "text");
		assert_eq!(hash_storage.chart_length, 64);
		assert_eq!(hash_storage.asset_type, "text");
		assert_eq!(hash_storage.asset_length, 64);
	}
}
