use backbeat_core::AssetId;
use backbeat_core::BundleId;
use std::path::PathBuf;

/// The content of an asset, resolved to either in-memory bytes or a filesystem path.
#[derive(Debug, Clone)]
pub enum AssetData {
	Bytes(Vec<u8>),
	File(PathBuf),
}

/// A bundle that references an asset.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetDependent {
	pub bundle_id: BundleId,
	pub description: String,
	pub path: String,
}

/// Full detail for a single asset.
#[derive(Debug, Clone)]
pub struct AssetDetail {
	pub id: AssetId,
	/// Size in bytes, if the asset is downloaded.
	pub size: Option<u64>,
	/// Every bundle known to depend on this asset, whether or not the asset
	/// itself has actually been downloaded.
	pub dependents: Vec<AssetDependent>,
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{AssetPath, ChartDesc, ChartFilename, Sha256};
	use backbeat_core::{BackbeatFile, ChartData};

	use super::*;
	use crate::StoreError;

	#[test]
	fn asset_detail_returns_not_found_for_unknown_asset() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_asset_detail_missing");

		let unknown = AssetId(Sha256::checksum_bytes(b"never seen"));
		assert!(matches!(
			store.asset_detail(unknown),
			Err(StoreError::NotFound(_))
		));
	}

	#[test]
	fn asset_detail_reports_size_and_dependents() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_asset_detail");

		let data = b"shared kick sample";
		let asset_id = AssetId(Sha256::checksum_bytes(data));
		crate::test_support::store_asset_unchecked(&store, asset_id, data).expect("store_asset");

		let mut assets_a = HashMap::new();
		assets_a.insert(AssetPath::from_path("sound/kick.ogg").unwrap(), asset_id);
		let bundle_a = store
			.import_bundle(&BackbeatFile {
				filename: ChartFilename::from_path("a.bms").unwrap(),
				assets: assets_a,
				desc: ChartDesc::new("test").unwrap(),
				chart: ChartData::compress(b"#ARTIST A\n#TITLE Song A\n").unwrap(),
			})
			.expect("import a");

		let mut assets_b = HashMap::new();
		assets_b.insert(AssetPath::from_path("drums/kick.ogg").unwrap(), asset_id);
		let bundle_b = store
			.import_bundle(&BackbeatFile {
				filename: ChartFilename::from_path("b.bms").unwrap(),
				assets: assets_b,
				desc: ChartDesc::new("test").unwrap(),
				chart: ChartData::compress(b"#ARTIST B\n#TITLE Song B\n").unwrap(),
			})
			.expect("import b");

		let detail = store.asset_detail(asset_id).expect("asset should exist");

		assert_eq!(detail.size, Some(data.len() as u64));
		assert_eq!(detail.dependents.len(), 2);

		let bundle_ids: Vec<BundleId> = detail.dependents.iter().map(|d| d.bundle_id).collect();
		assert!(bundle_ids.contains(&bundle_a));
		assert!(bundle_ids.contains(&bundle_b));
	}

	#[test]
	fn asset_detail_reports_dependents_with_no_size_when_never_downloaded() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_asset_detail_missing_size");

		let never_stored = AssetId(Sha256::checksum_bytes(b"never actually stored"));
		let mut assets = HashMap::new();
		assets.insert(
			AssetPath::from_path("sound/kick.ogg").unwrap(),
			never_stored,
		);

		store
			.import_bundle(&BackbeatFile {
				filename: ChartFilename::from_path("song.bms").unwrap(),
				assets,
				desc: ChartDesc::new("test").unwrap(),
				chart: ChartData::compress(b"#TITLE Test;").unwrap(),
			})
			.expect("import_bb");

		let detail = store
			.asset_detail(never_stored)
			.expect("asset should exist (referenced, even if never downloaded)");

		assert_eq!(detail.size, None);
		assert_eq!(detail.dependents.len(), 1);
	}
}
