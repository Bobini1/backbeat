#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::time::Duration;

use backbeat_core::{
	AssetId, AssetPath, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};
use backbeat_store_config::{BackbeatConfig, DownloadsConfig, ServerConfig};

mod support;
use support::{new_test_store_with, spawn_delayed_http_server};

const ASSET_COUNT: usize = 1_000;
const DOWNLOAD_CONCURRENCY: u32 = 32;

#[tokio::test(flavor = "multi_thread")]
async fn thousand_dependency_bundle_keeps_work_and_database_access_bounded() {
	let mut assets = backbeat_core::Assets::with_capacity(ASSET_COUNT);
	let mut routes = Vec::with_capacity(ASSET_COUNT + 1);

	for index in 0..ASSET_COUNT {
		let mut data = vec![0_u8; 4 * 1024];
		data[..8].copy_from_slice(&(index as u64).to_le_bytes());
		let asset_id = AssetId(Sha256::checksum_bytes(&data));
		assets.insert(
			AssetPath::from_path(&format!("{index:04}.wav")).unwrap(),
			asset_id,
		);
		routes.push((format!("/assets/{asset_id}"), data));
	}

	let bb = BackbeatFile {
		filename: ChartFilename::from_path("stress.bms").unwrap(),
		assets,
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE stress\n#BPM 120\n#00111:0100\n").unwrap(),
	};
	let bundle_id = bb.bundle_id();
	routes.push((format!("/bundles/{bundle_id}"), bb.to_json()));

	let server = spawn_delayed_http_server(routes, true, Duration::from_millis(2)).await;
	let (_tmp, store) = new_test_store_with(
		"bb_thousand_dependencies",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			downloads: DownloadsConfig {
				concurrency: DOWNLOAD_CONCURRENCY,
				..DownloadsConfig::default()
			},
			..BackbeatConfig::default()
		},
	);

	let download_store = store.clone();
	let download =
		tokio::spawn(async move { download_store.server_download_bundle(bundle_id).await });
	let mut max_registered = 0;
	let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

	while !download.is_finished() {
		assert!(
			tokio::time::Instant::now() < deadline,
			"pathological bundle download timed out"
		);
		let overview = store.download_overview();
		max_registered = max_registered.max(overview.total);
		store
			.search_bundles(None, 0, 50, &[])
			.expect("catalogue reads must remain responsive during download");
		tokio::time::sleep(Duration::from_millis(5)).await;
	}

	download
		.await
		.expect("download task panicked")
		.expect("pathological bundle download failed");

	assert!(
		max_registered <= (DOWNLOAD_CONCURRENCY * 2 + 1) as u64,
		"dependency count leaked into live work: saw {max_registered} registered downloads"
	);

	let stats = store.stats().unwrap();
	assert_eq!(stats.asset_count, ASSET_COUNT as u64);
	assert_eq!(stats.charts, 1);
}
