#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! Remote client path smoke tests against a minimal mock BDS.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use backbeat_core::{AssetId, BundleId, Sha256};
use backbeat_store_config::{BackbeatConfig, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

mod support;
use support::new_test_store_with;

#[tokio::test(flavor = "multi_thread")]
async fn remote_client_uses_bds_asset_and_bundle_paths() {
	let paths = Arc::new(Mutex::new(Vec::new()));
	let hits = Arc::new(AtomicUsize::new(0));
	let server = spawn_path_recording_server(Arc::clone(&paths), Arc::clone(&hits)).await;

	let config = BackbeatConfig {
		servers: vec![ServerConfig { url: server }],
		..BackbeatConfig::default()
	};
	let (_tmp, store) = new_test_store_with("bb_remote_paths", config);

	let asset_id = AssetId(Sha256::checksum_bytes(b"remote fixture"));
	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("remote asset check")
	);

	let bundle_id: BundleId = format!("b-{}", "1".repeat(64)).parse().unwrap();
	assert!(
		store
			.server_has_bundle(bundle_id)
			.await
			.expect("remote bundle check")
	);

	assert_eq!(hits.load(Ordering::SeqCst), 2);
	let recorded = paths.lock().await.clone();
	assert_eq!(recorded.len(), 2);
	assert!(
		recorded[0].starts_with("/assets/"),
		"expected /assets/ path, got {}",
		recorded[0]
	);
	assert!(
		recorded[1].starts_with("/bundles/"),
		"expected /bundles/ path, got {}",
		recorded[1]
	);
	assert!(
		!recorded[0].starts_with("/asset/"),
		"singular /asset/ must not be used: {}",
		recorded[0]
	);
	assert!(
		!recorded[1].starts_with("/bundle/"),
		"singular /bundle/ must not be used: {}",
		recorded[1]
	);
}

async fn spawn_path_recording_server(
	paths: Arc<Mutex<Vec<String>>>,
	hits: Arc<AtomicUsize>,
) -> String {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();

	tokio::spawn(async move {
		loop {
			let Ok((mut stream, _)) = listener.accept().await else {
				break;
			};
			let paths = Arc::clone(&paths);
			let hits = Arc::clone(&hits);
			tokio::spawn(async move {
				let mut buf = [0u8; 2048];
				let n = stream.read(&mut buf).await.unwrap_or(0);
				hits.fetch_add(1, Ordering::SeqCst);
				let request = String::from_utf8_lossy(&buf[..n]);
				let path = request
					.lines()
					.next()
					.and_then(|line| line.split_whitespace().nth(1))
					.unwrap_or("")
					.to_string();
				paths.lock().await.push(path);

				let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
				let _ = stream.write_all(response.as_bytes()).await;
			});
		}
	});

	format!("http://{addr}")
}
