#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::cell::RefCell;
use std::rc::Rc;

use backbeat_core::{
	AssetId, AssetPath, Assets, BackbeatFile, ChartData, ChartDesc, ChartFilename, Sha256,
};
use backbeat_store_config::{BackbeatConfig, ServerConfig};
use serde_json::json;

mod support;

struct Fixture {
	_server_runtime: tokio::runtime::Runtime,
	url: String,
	bundle: BackbeatFile,
	asset_id: AssetId,
}

impl Fixture {
	fn new() -> Self {
		let server_runtime = tokio::runtime::Builder::new_multi_thread()
			.worker_threads(1)
			.enable_all()
			.build()
			.unwrap();
		let data = b"runtime independent asset".to_vec();
		let asset_id = AssetId(Sha256::checksum_bytes(&data));
		let bundle = BackbeatFile {
			filename: ChartFilename::new("runtime.bms").unwrap(),
			assets: Assets::from([(AssetPath::new("song.wav").unwrap(), asset_id)]),
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#PLAYER 1\n#TITLE Runtime\n#BPM 120\n#00111:0100\n")
				.unwrap(),
		};
		let chart_id = format!("sha256/{}", bundle.chart_sha256());
		let url = server_runtime.block_on(support::spawn_http_server(
			vec![
				(format!("/assets/{asset_id}"), data),
				(format!("/bundles/{}", bundle.bundle_id()), bundle.to_json()),
				(format!("/charts/{chart_id}"), bundle.to_json()),
				(
					"/pack/header.json".into(),
					json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "pack"})
						.to_string()
						.into_bytes(),
				),
				(
					"/pack/data.bbpack".into(),
					json!({
						"name": "Runtime Pack", "gamemode": "bms-7k",
						"updated": "2026-07-15T00:00:00Z", "tags": {}, "assets": {},
						"bundles": [{"id": bundle.bundle_id(), "desc": "Runtime", "tags": {}}]
					})
					.to_string()
					.into_bytes(),
				),
			],
			true,
		));
		Self {
			_server_runtime: server_runtime,
			url,
			bundle,
			asset_id,
		}
	}

	async fn exercise(&self) {
		let (_temp, store) = support::new_test_store_with(
			"runtime_independence",
			BackbeatConfig {
				servers: vec![ServerConfig {
					url: self.url.clone(),
				}],
				..BackbeatConfig::default()
			},
		);
		let bundle_id = self.bundle.bundle_id();
		let chart_id = format!("sha256/{}", self.bundle.chart_sha256())
			.parse()
			.unwrap();
		assert!(store.server_has_asset(self.asset_id).await.unwrap());
		assert!(store.server_has_bundle(bundle_id).await.unwrap());
		assert!(store.server_has_chart(&chart_id).await.unwrap());

		let url = format!("{}/pack", self.url);
		store.collection_fetch_header(&url).await.unwrap();
		store.collection_fetch_upsert(&url).await.unwrap();
		let caller_thread = std::thread::current().id();
		let progress = Rc::new(RefCell::new(Vec::new()));
		let report = store
			.collection_fetch_download_data(&url, |update| {
				assert_eq!(std::thread::current().id(), caller_thread);
				assert!(store.has_bundle(bundle_id).unwrap());
				progress.borrow_mut().push(update.current);
			})
			.await
			.unwrap();
		assert!(!report.has_failures(), "{report:?}");
		assert_eq!(report.bundles_downloaded, 1);
		assert_eq!(*progress.borrow(), vec![1]);
		assert!(store.has_asset(self.asset_id).unwrap());

		store.server_download_asset(self.asset_id).await.unwrap();
		store.server_download_bundle(bundle_id).await.unwrap();
		store.server_download_chart(&chart_id).await.unwrap();
		for _ in 0..64 {
			assert!(store.server_has_asset(self.asset_id).await.unwrap());
			assert!(store.has_bundle(bundle_id).unwrap());
		}
	}
}

#[test]
fn network_and_database_work_without_a_caller_tokio_runtime() {
	let fixture = Fixture::new();
	assert!(tokio::runtime::Handle::try_current().is_err());
	futures::executor::block_on(fixture.exercise());
}

#[test]
fn network_and_database_work_on_current_thread_without_io_or_timers() {
	let fixture = Fixture::new();
	let caller = tokio::runtime::Builder::new_current_thread()
		.build()
		.unwrap();
	caller.block_on(fixture.exercise());
}

#[test]
fn network_and_database_work_on_multi_thread_without_io_or_timers() {
	let fixture = Fixture::new();
	let caller = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(1)
		.build()
		.unwrap();
	caller.block_on(fixture.exercise());
}

#[test]
fn synchronous_queue_keeps_downloading_without_a_caller_executor() {
	let fixture = Fixture::new();
	let (_temp, store) = support::new_test_store_with(
		"runtime_sync_queue",
		BackbeatConfig {
			servers: vec![ServerConfig {
				url: fixture.url.clone(),
			}],
			..BackbeatConfig::default()
		},
	);
	let bundle_id = store.import_bundle(&fixture.bundle).unwrap();
	store.bundle_download_assets(bundle_id).unwrap();
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
	while !store.has_asset(fixture.asset_id).unwrap() {
		assert!(
			std::time::Instant::now() < deadline,
			"queued download stalled"
		);
		std::thread::sleep(std::time::Duration::from_millis(5));
	}
}
