#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::collections::HashMap;
use std::fs::{File, FileTimes};
use std::time::{Duration, SystemTime};

use backbeat_core::{
	AssetId, AssetPath, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};
use backbeat_sdk::{AssetData, Backbeat};
use backbeat_store_config::{BackbeatConfig, ByteSize};
use serde_json::json;

mod support;
use support::{new_test_store, new_test_store_with, spawn_collection_server};

fn import_asset(store: &Backbeat, root: &std::path::Path, name: &str, data: &[u8]) -> AssetId {
	let source = root.join(name);
	fs_err::write(&source, data).expect("write source asset");
	store.import_asset(&source).expect("import asset");
	AssetId(Sha256::checksum_bytes(data))
}

#[test]
fn asset_prune_is_dry_run_safe_and_preserves_bundle_assets() {
	let mut config = BackbeatConfig::default();
	config.store.inline = ByteSize(8);
	let (temp, store) = new_test_store_with("asset_prune", config);
	let inline = import_asset(&store, temp.path(), "inline", b"small");
	let file = import_asset(&store, temp.path(), "file", b"unused file asset");
	let referenced = import_asset(&store, temp.path(), "referenced", b"referenced file asset");
	let file_path = match store.get_asset(file).unwrap() {
		AssetData::File(path) => path,
		AssetData::Bytes(_) => panic!("expected a file-backed asset"),
	};
	let referenced_path = match store.get_asset(referenced).unwrap() {
		AssetData::File(path) => path,
		AssetData::Bytes(_) => panic!("expected a file-backed asset"),
	};

	store
		.import_bundle(&BackbeatFile {
			filename: ChartFilename::new("referenced.bms").unwrap(),
			assets: HashMap::from([(AssetPath::new("referenced.wav").unwrap(), referenced)]),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#TITLE Referenced\n").unwrap(),
		})
		.unwrap();
	let revision = store.should_refresh(-1).unwrap().1;

	assert_eq!(store.asset_prune(true).unwrap(), 2);
	assert!(store.has_asset(inline).unwrap());
	assert!(store.has_asset(file).unwrap());
	assert!(!store.should_refresh(revision).unwrap().0);

	assert_eq!(store.asset_prune(false).unwrap(), 2);
	assert!(!store.has_asset(inline).unwrap());
	assert!(!store.has_asset(file).unwrap());
	assert!(!file_path.exists());
	assert!(store.has_asset(referenced).unwrap());
	assert!(referenced_path.exists());
	assert!(store.should_refresh(revision).unwrap().0);
	assert_eq!(store.asset_prune(false).unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn asset_prune_preserves_collection_assets() {
	let (temp, store) = new_test_store("asset_prune_collections");
	let table = import_asset(&store, temp.path(), "table", b"table asset");
	let course = import_asset(&store, temp.path(), "course", b"course asset");
	let pack = import_asset(&store, temp.path(), "pack", b"pack asset");
	let orphan = import_asset(&store, temp.path(), "orphan", b"orphan asset");
	let timestamp = "2026-07-15T00:00:00Z";
	let routes = vec![
		(
			"/table/header.json".to_owned(),
			json!({"timestamp": timestamp, "kind": "table"}).to_string(),
		),
		(
			"/table/data.bbtable".to_owned(),
			json!({"name": "Table", "symbol": "T", "gamemode": "bms-7k", "updated": timestamp, "tags": {}, "assets": {"banner": table}, "levels": [], "folders": []}).to_string(),
		),
		(
			"/course/header.json".to_owned(),
			json!({"timestamp": timestamp, "kind": "course"}).to_string(),
		),
		(
			"/course/data.bbcourse".to_owned(),
			json!({"name": "Course", "gamemode": "bms-7k", "updated": timestamp, "tags": {}, "assets": {"banner": course}, "charts": []}).to_string(),
		),
		(
			"/pack/header.json".to_owned(),
			json!({"timestamp": timestamp, "kind": "pack"}).to_string(),
		),
		(
			"/pack/data.bbpack".to_owned(),
			json!({"name": "Pack", "gamemode": "bms-7k", "updated": timestamp, "tags": {}, "assets": {"banner": pack}, "bundles": []}).to_string(),
		),
	];
	let base = spawn_collection_server(routes).await;

	for kind in ["table", "course", "pack"] {
		store
			.collection_fetch_upsert(&format!("{base}/{kind}"))
			.await
			.unwrap();
	}

	assert_eq!(store.asset_prune(false).unwrap(), 1);
	assert!(store.has_asset(table).unwrap());
	assert!(store.has_asset(course).unwrap());
	assert!(store.has_asset(pack).unwrap());
	assert!(!store.has_asset(orphan).unwrap());
}

#[test]
fn disk_prune_removes_orphans_and_stale_downloads() {
	let mut config = BackbeatConfig::default();
	config.store.inline = ByteSize::ZERO;
	let (temp, store) = new_test_store_with("disk_prune", config);
	let known = import_asset(&store, temp.path(), "known", b"known asset");
	let known_path = match store.get_asset(known).unwrap() {
		AssetData::File(path) => path,
		AssetData::Bytes(_) => panic!("expected a file-backed asset"),
	};

	let orphan = AssetId(Sha256::checksum_bytes(b"orphan file"));
	let orphan_path = store
		.store_dir()
		.join(Backbeat::ASSETS_DIR)
		.join(orphan.fanned_path());
	fs_err::create_dir_all(orphan_path.parent().unwrap()).unwrap();
	fs_err::write(&orphan_path, b"orphan file").unwrap();
	let malformed = store.store_dir().join(Backbeat::ASSETS_DIR).join("README");
	fs_err::write(&malformed, b"leave me alone").unwrap();

	let downloads = store.store_dir().join(Backbeat::DOWNLOADING_DIR);
	let stale = downloads.join("stale");
	let recent = downloads.join("recent");
	let stale_file = File::create(&stale).unwrap();
	File::create(&recent).unwrap();
	stale_file
		.set_times(
			FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(2 * 60 * 60)),
		)
		.unwrap();

	assert_eq!(store.disk_prune(true).unwrap(), 2);
	assert!(orphan_path.exists());
	assert!(stale.exists());

	assert_eq!(store.disk_prune(false).unwrap(), 2);
	assert!(known_path.exists());
	assert!(!orphan_path.exists());
	assert!(!stale.exists());
	assert!(recent.exists());
	assert!(malformed.exists());
	assert_eq!(store.disk_prune(false).unwrap(), 0);
}
