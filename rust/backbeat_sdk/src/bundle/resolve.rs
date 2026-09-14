#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{AssetId, AssetPath, ChartData, ChartDesc, ChartFilename, Sha256};
	use backbeat_store_config::{BackbeatConfig, ByteSize};

	use backbeat_core::BackbeatFile;

	use crate::AssetData;
	use crate::StoreError;

	fn bundle(asset_id: AssetId) -> BackbeatFile {
		BackbeatFile {
			filename: ChartFilename::from_path("song.bms").unwrap(),
			assets: HashMap::from([(AssetPath::from_path("Audio/Song.ogg").unwrap(), asset_id)]),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"chart").unwrap(),
		}
	}

	fn resolve(
		store: &crate::Backbeat,
		bundle: &BackbeatFile,
		path: &str,
	) -> Result<Option<AssetData>, StoreError> {
		bundle
			.resolve_path(path)
			.map(|asset| store.get_asset(asset))
			.transpose()
	}

	#[test]
	fn resolves_inline_asset() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_bundle_resolve_inline");
		let data = b"inline asset";
		let asset_id = AssetId(Sha256::checksum_bytes(data));
		crate::test_support::store_asset_unchecked(&store, asset_id, data).unwrap();

		let resolved = resolve(&store, &bundle(asset_id), r"audio\song.OGG").unwrap();

		assert!(matches!(
			resolved,
			Some(AssetData::Bytes(bytes)) if bytes == data
		));
	}

	#[test]
	fn resolves_file_asset() {
		let mut config = BackbeatConfig::default();
		config.store.inline = ByteSize::ZERO;
		let (_tmp, store) = crate::test_util::new_test_store_with("bb_bundle_resolve_file", config);
		let data = b"file asset";
		let asset_id = AssetId(Sha256::checksum_bytes(data));
		crate::test_support::store_asset_unchecked(&store, asset_id, data).unwrap();

		let resolved = resolve(&store, &bundle(asset_id), "Audio/Song.ogg").unwrap();

		let Some(AssetData::File(path)) = resolved else {
			panic!("expected file-backed asset");
		};
		assert_eq!(std::fs::read(path).unwrap(), data);
	}

	#[test]
	fn returns_none_for_unresolved_path() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_bundle_resolve_missing_path");
		let asset_id = AssetId(Sha256::checksum_bytes(b"asset"));

		assert!(
			resolve(&store, &bundle(asset_id), "missing.ogg")
				.unwrap()
				.is_none()
		);
	}

	#[test]
	fn returns_error_for_unstored_resolved_asset() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_bundle_resolve_unstored");
		let asset_id = AssetId(Sha256::checksum_bytes(b"unstored asset"));

		assert!(matches!(
			resolve(&store, &bundle(asset_id), "Audio/Song.ogg"),
			Err(StoreError::NotFound(_))
		));
	}
}
