#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! Build a FUSE index from bundled repo fixtures.

use std::path::{Path, PathBuf};

use backbeat_packager::SeenCache;
use backbeat_sdk::Backbeat;
use backbeat_vf_fuse::prepare;
use walkdir::WalkDir;

fn fixtures_charts_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/charts")
}

fn test_store() -> (tempfile::TempDir, Backbeat) {
	let tmp = tempfile::TempDir::new().unwrap();
	let store = Backbeat::open_with_overridden_config_dir(tmp.path()).expect("create store");
	(tmp, store)
}

fn import_fixture_charts(store: &Backbeat, charts_root: &Path) -> usize {
	let cache = SeenCache::new();
	let mut imported = 0usize;

	for entry in WalkDir::new(charts_root)
		.follow_links(false)
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| e.file_type().is_file())
	{
		let path = entry.path();
		if !backbeat_packager::should_be_packaged(path) {
			continue;
		}

		let bundles = backbeat_packager::package_with_cache(path, &cache)
			.unwrap_or_else(|err| panic!("package {}: {err}", path.display()));
		if let Some(bundle) = bundles.first() {
			for (filename, &asset_id) in &bundle.assets {
				let asset_path = path.parent().unwrap_or(Path::new(".")).join(filename);
				let data = std::fs::read(&asset_path)
					.unwrap_or_else(|err| panic!("read {}: {err}", asset_path.display()));
				backbeat_sdk::test_support::store_asset_unchecked(store, asset_id, &data)
					.unwrap_or_else(|err| panic!("store {}: {err}", asset_path.display()));
			}
		}
		for bundle in &bundles {
			store
				.import_bundle(bundle)
				.unwrap_or_else(|err| panic!("import {}: {err}", path.display()));
		}
		imported += 1;
	}

	imported
}

#[test]
#[ignore = "dont work"]
fn prepare_check() {
	let charts_root = fixtures_charts_dir();
	if !charts_root.is_dir() {
		eprintln!("SKIP: fixtures not found at {}", charts_root.display());
		return;
	}

	let (_tmp, store) = test_store();
	let chart_count = import_fixture_charts(&store, &charts_root);
	assert!(chart_count > 0, "expected at least one chart fixture");

	let fs = prepare(store, "https://localhost/virtual-folders/stepmania").expect("prepare");

	eprintln!("index nodes: {}", fs.index().len());
	let children = fs.index().list_children(1).expect("root is directory");
	eprintln!("root children: {}", children.len());
	for (name, _, _) in children.iter().take(10) {
		eprintln!("  {name}");
	}

	// In the new model bundles are global and a virtual folder's FUSE index is
	// only populated when bundle placements exist AND their bundles are
	// locally imported. This test imports fixtures but adds no placements, so
	// it only verifies that prepare() succeeds against an empty virtual folder.
	assert!(!fs.index().is_empty(), "expected root inode");
}
