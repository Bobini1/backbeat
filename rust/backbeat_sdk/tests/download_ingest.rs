#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;

use backbeat_core::asset_id::AssetId;
use backbeat_core::{
	AssetPath, Assets, BackbeatFile, ChartData, ChartDesc, ChartFilename, CustomIdAlgorithm,
	IdAlgorithm, Sha256,
};
use backbeat_sdk::StoreError;
use backbeat_store_config::{BackbeatConfig, ServerConfig};
use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, EntryType, Header};

mod support;
use support::{new_test_store_with, spawn_http_server, spawn_mutable_collection_server};

#[tokio::test(flavor = "multi_thread")]
async fn installed_bundle_can_queue_missing_assets() {
	let data = b"queued asset".to_vec();
	let asset_id = AssetId(Sha256::checksum_bytes(&data));
	let bundle = BackbeatFile {
		filename: ChartFilename::from_path("queued.bms").unwrap(),
		assets: Assets::from([(AssetPath::from_path("queued.wav").unwrap(), asset_id)]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE queued\n#BPM 120\n#00111:0100\n").unwrap(),
	};
	let server = spawn_http_server(vec![(format!("/assets/{asset_id}"), data)], true).await;
	let (_tmp, store) = new_test_store_with(
		"bb_queue_bundle_assets",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);
	let bundle_id = store.import_bundle(&bundle).unwrap();

	assert!(!store.has_asset(asset_id).unwrap());
	store.bundle_download_assets(bundle_id).unwrap();

	tokio::time::timeout(Duration::from_secs(5), async {
		while !store.has_asset(asset_id).unwrap() {
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("queued asset download should finish");
}

#[tokio::test(flavor = "multi_thread")]
async fn small_known_size_downloads_without_staging_residue() {
	let data = b"small-inline-payload";
	let asset_id = AssetId(Sha256::checksum_bytes(data));
	let server =
		spawn_http_server(vec![(format!("/assets/{asset_id}"), data.to_vec())], true).await;
	let (_tmp, store) = new_test_store_with(
		"bb_ingest_inline",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_asset(asset_id)
		.await
		.expect("download inline");

	assert!(store.has_asset(asset_id).unwrap());
	let staging = store
		.store_dir()
		.join(".downloading")
		.join(asset_id.to_string());
	assert!(
		!staging.exists(),
		"inline ingest should not leave a staging file at {}",
		staging.display()
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn known_size_above_inline_threshold_stays_out_of_sqlite() {
	let data = vec![7_u8; 128 * 1024 + 1];
	let asset_id = AssetId(Sha256::checksum_bytes(&data));
	let server = spawn_http_server(vec![(format!("/assets/{asset_id}"), data)], true).await;
	let (_tmp, store) = new_test_store_with(
		"bb_ingest_known_size_fs",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_asset(asset_id)
		.await
		.expect("download filesystem asset");

	let stats = store.stats().unwrap();
	assert_eq!(stats.asset_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn bundle_downloads_every_asset_type() {
	let audio_data = b"song".to_vec();
	let video_data = b"movie".to_vec();
	let image_data = b"jacket".to_vec();
	let other_data = b"bga".to_vec();
	let audio_id = AssetId(Sha256::checksum_bytes(&audio_data));
	let video_id = AssetId(Sha256::checksum_bytes(&video_data));
	let image_id = AssetId(Sha256::checksum_bytes(&image_data));
	let other_id = AssetId(Sha256::checksum_bytes(&other_data));
	let bundle = BackbeatFile {
		filename: ChartFilename::from_path("filter.bms").unwrap(),
		assets: Assets::from([
			(AssetPath::from_path("song.ogg").unwrap(), audio_id),
			(AssetPath::from_path("movie.mp4").unwrap(), video_id),
			(AssetPath::from_path("jacket.png").unwrap(), image_id),
			(AssetPath::from_path("background.bga").unwrap(), other_id),
		]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE filter\n#BPM 120\n#00111:0100\n").unwrap(),
	};
	let bundle_id = bundle.bundle_id();
	let server = spawn_http_server(
		vec![
			(format!("/bundles/{bundle_id}"), bundle.to_json()),
			(format!("/assets/{audio_id}"), audio_data),
			(format!("/assets/{video_id}"), video_data),
			(format!("/assets/{image_id}"), image_data),
			(format!("/assets/{other_id}"), other_data),
		],
		true,
	)
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_download_all_asset_types",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_bundle(bundle_id)
		.await
		.expect("download bundle with every dependency");

	assert!(store.has_bundle(bundle_id).unwrap());
	assert!(store.has_asset(audio_id).unwrap());
	assert!(store.has_asset(video_id).unwrap());
	assert!(store.has_asset(image_id).unwrap());
	assert!(store.has_asset(other_id).unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_asset_download_does_not_commit_bundle_metadata() {
	let asset_data = "retry asset";
	let asset_id = AssetId(Sha256::checksum_bytes(asset_data.as_bytes()));
	let bundle = BackbeatFile {
		filename: ChartFilename::from_path("retry.bms").unwrap(),
		assets: Assets::from([(AssetPath::from_path("retry.wav").unwrap(), asset_id)]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE retry\n#BPM 120\n#00111:0100\n").unwrap(),
	};
	let bundle_id = bundle.bundle_id();
	let (server, routes) = spawn_mutable_collection_server(vec![(
		format!("/bundles/{bundle_id}"),
		String::from_utf8(bundle.to_json()).unwrap(),
	)])
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_retry_bundle_after_asset_failure",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	assert!(store.server_download_bundle(bundle_id).await.is_err());
	assert!(!store.has_bundle(bundle_id).unwrap());
	assert!(!store.has_asset(asset_id).unwrap());

	routes
		.lock()
		.await
		.insert(format!("/assets/{asset_id}"), asset_data.to_owned());
	store
		.server_download_bundle(bundle_id)
		.await
		.expect("retry after the asset becomes available");

	assert!(store.has_bundle(bundle_id).unwrap());
	assert!(store.has_asset(asset_id).unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn bundle_uses_precombined_assets_at_the_threshold() {
	let (bundle, data) = bundle_with_assets(30);
	let bundle_id = bundle.bundle_id();
	let archive = precombined_archive(&bundle.assets, &data);
	let server = spawn_http_server(
		vec![
			(format!("/bundles/{bundle_id}"), bundle.to_json()),
			(
				format!("/precombined-assets/{}", bundle.combined_assets_id()),
				archive,
			),
		],
		true,
	)
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_precombined_assets",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_bundle(bundle_id)
		.await
		.expect("download bundle from precombined archive");

	for asset_id in bundle.assets.values() {
		assert!(store.has_asset(*asset_id).unwrap());
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_precombined_assets_falls_back_to_individual_assets() {
	let (bundle, data) = bundle_with_assets(30);
	let bundle_id = bundle.bundle_id();
	let mut routes = vec![(format!("/bundles/{bundle_id}"), bundle.to_json())];
	routes.extend(data.values().map(|bytes| {
		(
			format!("/assets/{}", AssetId(Sha256::checksum_bytes(bytes))),
			bytes.clone(),
		)
	}));
	let server = spawn_http_server(routes, true).await;
	let (_tmp, store) = new_test_store_with(
		"bb_precombined_assets_fallback",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_bundle(bundle_id)
		.await
		.expect("fall back to individual assets");

	for asset_id in bundle.assets.values() {
		assert!(store.has_asset(*asset_id).unwrap());
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_precombined_assets_falls_back_to_individual_assets() {
	let (bundle, data) = bundle_with_assets(30);
	let bundle_id = bundle.bundle_id();
	let mut routes = vec![
		(format!("/bundles/{bundle_id}"), bundle.to_json()),
		(
			format!("/precombined-assets/{}", bundle.combined_assets_id()),
			b"not a gzip archive".to_vec(),
		),
	];
	routes.extend(data.values().map(|bytes| {
		(
			format!("/assets/{}", AssetId(Sha256::checksum_bytes(bytes))),
			bytes.clone(),
		)
	}));
	let server = spawn_http_server(routes, true).await;
	let (_tmp, store) = new_test_store_with(
		"bb_precombined_assets_malformed_fallback",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_bundle(bundle_id)
		.await
		.expect("fall back after malformed precombined archive");

	for asset_id in bundle.assets.values() {
		assert!(store.has_asset(*asset_id).unwrap());
	}
}

fn bundle_with_assets(count: usize) -> (BackbeatFile, HashMap<AssetPath, Vec<u8>>) {
	let mut assets = Assets::new();
	let mut data = HashMap::new();
	for index in 0..count {
		let path = AssetPath::from_path(&format!("assets/{index:02}.bin")).unwrap();
		let bytes = format!("asset {index}").into_bytes();
		assets.insert(path.clone(), AssetId(Sha256::checksum_bytes(&bytes)));
		data.insert(path, bytes);
	}

	(
		BackbeatFile {
			filename: ChartFilename::from_path("precombined.bms").unwrap(),
			assets,
			desc: ChartDesc::new("test").unwrap(),
			chart: ChartData::compress(b"#PLAYER 1\n#TITLE precombined\n#BPM 120\n#00111:0100\n")
				.unwrap(),
		},
		data,
	)
}

fn precombined_archive(assets: &Assets, data: &HashMap<AssetPath, Vec<u8>>) -> Vec<u8> {
	let encoder = GzEncoder::new(Vec::new(), Compression::default());
	let mut archive = Builder::new(encoder);
	for (path, asset_id) in assets {
		let bytes = data.get(path).unwrap();
		assert_eq!(*asset_id, AssetId(Sha256::checksum_bytes(bytes)));

		let mut header = Header::new_gnu();
		header.set_entry_type(EntryType::Regular);
		header.set_size(bytes.len() as u64);
		header.set_cksum();
		archive
			.append_data(&mut header, path.as_str(), Cursor::new(bytes))
			.unwrap();
	}
	archive.into_inner().unwrap().finish().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_size_download_cleans_up_staging_file() {
	let data = b"staged-small-payload";
	let asset_id = AssetId(Sha256::checksum_bytes(data));
	let server =
		spawn_http_server(vec![(format!("/assets/{asset_id}"), data.to_vec())], false).await;
	let (_tmp, store) = new_test_store_with(
		"bb_ingest_staged",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	store
		.server_download_asset(asset_id)
		.await
		.expect("download staged");

	assert!(store.has_asset(asset_id).unwrap());
	let staging = store
		.store_dir()
		.join(".downloading")
		.join(asset_id.to_string());
	assert!(
		!staging.exists(),
		"staging file should be cleaned up after commit"
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn inline_download_rejects_hash_mismatch() {
	let expected = AssetId(Sha256::checksum_bytes(b"expected"));
	let server = spawn_http_server(
		vec![(format!("/assets/{expected}"), b"wrong!!!".to_vec())],
		true,
	)
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_ingest_bad_hash",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	let err = store.server_download_asset(expected).await.unwrap_err();
	assert!(matches!(&*err, StoreError::HashMismatch));
}

#[tokio::test(flavor = "multi_thread")]
async fn bundle_download_rejects_mismatched_bundle_before_import() {
	let requested = BackbeatFile {
		filename: ChartFilename::from_path("requested.bms").unwrap(),
		assets: Assets::new(),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE requested\n#BPM 120\n#00111:0100\n")
			.unwrap(),
	};
	let requested_id = requested.bundle_id();

	let asset_data = b"wrong bundle asset".to_vec();
	let asset_id = AssetId(Sha256::checksum_bytes(&asset_data));
	let actual = BackbeatFile {
		filename: ChartFilename::from_path("actual.bms").unwrap(),
		assets: Assets::from([(AssetPath::from_path("actual.wav").unwrap(), asset_id)]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE actual\n#BPM 150\n#00111:0100\n").unwrap(),
	};
	let actual_id = actual.bundle_id();
	assert_ne!(requested_id, actual_id);

	let server = spawn_http_server(
		vec![
			(format!("/bundles/{requested_id}"), actual.to_json()),
			(format!("/assets/{asset_id}"), asset_data),
		],
		true,
	)
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_bundle_id_mismatch",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	let err = store
		.server_download_bundle(requested_id)
		.await
		.unwrap_err();
	assert!(matches!(
		&*err,
		StoreError::BundleIdMismatch { expected, actual }
			if *expected == requested_id && *actual == actual_id
	));

	assert!(!store.has_bundle(requested_id).unwrap());
	assert!(!store.has_bundle(actual_id).unwrap());
	assert!(!store.has_asset(asset_id).unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn chart_download_rejects_mismatched_requested_algorithm_before_import() {
	let algorithm = IdAlgorithm::Custom(CustomIdAlgorithm::new("md5").unwrap());
	let requested = BackbeatFile {
		filename: ChartFilename::from_path("requested.bms").unwrap(),
		assets: Assets::new(),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE requested\n#BPM 120\n#00111:0100\n")
			.unwrap(),
	};
	let requested_id = requested
		.additional_chart_ids()
		.into_iter()
		.find(|id| id.alg == algorithm)
		.unwrap();

	let asset_data = b"wrong chart asset".to_vec();
	let asset_id = AssetId(Sha256::checksum_bytes(&asset_data));
	let actual = BackbeatFile {
		filename: ChartFilename::from_path("actual.bms").unwrap(),
		assets: Assets::from([(AssetPath::from_path("actual.wav").unwrap(), asset_id)]),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(b"#PLAYER 1\n#TITLE actual\n#BPM 150\n#00111:0100\n").unwrap(),
	};
	let actual_id = actual
		.additional_chart_ids()
		.into_iter()
		.find(|id| id.alg == algorithm)
		.unwrap();
	let actual_bundle_id = actual.bundle_id();
	assert_ne!(requested_id, actual_id);

	let server = spawn_http_server(
		vec![
			(format!("/charts/{requested_id}"), actual.to_json()),
			(format!("/assets/{asset_id}"), asset_data),
		],
		true,
	)
	.await;
	let (_tmp, store) = new_test_store_with(
		"bb_chart_id_mismatch",
		BackbeatConfig {
			servers: vec![ServerConfig { url: server }],
			..BackbeatConfig::default()
		},
	);

	let err = store
		.server_download_chart(&requested_id)
		.await
		.unwrap_err();
	assert!(matches!(
		&*err,
		StoreError::ChartIdMismatch { expected, actual }
			if *expected == requested_id && *actual == actual_id
	));

	assert!(!store.has_chart(&requested_id).unwrap());
	assert!(!store.has_chart(&actual_id).unwrap());
	assert!(!store.has_bundle(actual_bundle_id).unwrap());
	assert!(!store.has_asset(asset_id).unwrap());
}
