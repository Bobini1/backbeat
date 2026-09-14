#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::collections::HashMap;

use backbeat_core::{
	AssetId, AssetPath, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};
use backbeat_sdk::StoreError;

mod support;
use support::new_test_store;

#[test]
fn uninstall_bundle_preserves_shared_data_until_last_bundle() {
	let (temp, store) = new_test_store("uninstall_bundle");
	let data = b"shared asset";
	let source = temp.path().join("asset.bin");
	fs_err::write(&source, data).unwrap();
	let asset_id = AssetId(Sha256::checksum_bytes(data));
	store.import_asset(&source).unwrap();

	let assets = HashMap::from([(AssetPath::new("sound.wav").unwrap(), asset_id)]);
	let chart = ChartData::compress(b"#TITLE Shared\n").unwrap();
	let first = store
		.import_bundle(&BackbeatFile {
			filename: ChartFilename::new("first.bms").unwrap(),
			assets: assets.clone(),
			desc: ChartDesc::new("test").unwrap(),
			chart: chart.clone(),
		})
		.unwrap();
	let second = store
		.import_bundle(&BackbeatFile {
			filename: ChartFilename::new("second.bms").unwrap(),
			assets,
			desc: ChartDesc::new("test").unwrap(),
			chart,
		})
		.unwrap();
	let chart_id = store.bundle_detail(first).unwrap().chart_sha256;
	let chart_id = format!("sha256/{chart_id}").parse().unwrap();

	store.bundle_rm(first).unwrap();
	assert!(!store.has_bundle(first).unwrap());
	assert!(store.has_bundle(second).unwrap());
	assert!(store.get_chart_data(&chart_id).is_ok());
	assert!(store.has_asset(asset_id).unwrap());

	store.bundle_rm(second).unwrap();
	assert!(!store.has_bundle(second).unwrap());
	assert!(matches!(
		store.get_chart_data(&chart_id),
		Err(StoreError::NotFound(_))
	));
	assert!(store.has_asset(asset_id).unwrap());
	assert_eq!(store.asset_prune(false).unwrap(), 1);
	assert!(matches!(
		store.get_asset(asset_id),
		Err(StoreError::NotFound(_))
	));
}
