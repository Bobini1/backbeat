#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use backbeat_core::{AssetId, Sha256};
use backbeat_sdk::AssetData;
use backbeat_store_config::{BackbeatConfig, ByteSize};

mod support;
use support::new_test_store_with;

#[test]
fn importing_large_asset_persists_file_and_download_record() {
	let mut config = BackbeatConfig::default();
	config.store.inline = ByteSize(8);
	let (temp, store) = new_test_store_with("bb_import_large_asset", config);
	let data = b"larger than the inline threshold";
	let source = temp.path().join("asset.bin");
	fs_err::write(&source, data).expect("write source asset");
	let asset_id = AssetId(Sha256::checksum_bytes(data));

	store.import_asset(&source).expect("import large asset");

	assert!(
		store.has_asset(asset_id).expect("check imported asset"),
		"the imported asset should have a downloaded_asset record"
	);
	match store.get_asset(asset_id).expect("read imported asset") {
		AssetData::File(path) => {
			assert_eq!(fs_err::read(path).expect("read stored asset"), data)
		}
		AssetData::Bytes(_) => panic!("large asset should be stored on the filesystem"),
	}
}
