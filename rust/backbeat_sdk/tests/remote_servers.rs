#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! Remote server ordering and fallback.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use backbeat_core::{AssetId, Sha256};
use backbeat_sdk::StoreError;
use backbeat_server_client::RemoteError;
use backbeat_store_config::{BackbeatConfig, CONFIG_FILENAME, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
mod support;
use support::new_test_store_with;

#[test]
fn remote_list_add_and_remove_updates_live_state_and_config() {
	let (_tmp, store) = new_test_store_with(
		"bb_remote_live_list",
		BackbeatConfig {
			servers: Vec::new(),
			..BackbeatConfig::default()
		},
	);
	let server = ServerConfig {
		url: "https://example.com".to_owned(),
	};
	let cloned_store = store.clone();

	assert!(store.has_zero_data_servers());
	store.server_add(&server).unwrap();
	assert!(!store.has_zero_data_servers());
	assert_eq!(store.config().servers, vec![server.clone()]);
	assert_eq!(cloned_store.config().servers, vec![server.clone()]);
	store.server_add(&server).unwrap();
	let config = BackbeatConfig::load_with_overridden_dir(store.config_dir()).unwrap();
	assert_eq!(config.servers, vec![server.clone()]);

	store.server_rm(&server).unwrap();
	assert!(store.has_zero_data_servers());
	let config = BackbeatConfig::load_with_overridden_dir(store.config_dir()).unwrap();
	assert!(config.servers.is_empty());
}

#[test]
fn failed_remote_config_write_does_not_change_live_or_in_memory_state() {
	let (_tmp, store) = new_test_store_with(
		"bb_remote_config_write_failure",
		BackbeatConfig {
			servers: Vec::new(),
			..BackbeatConfig::default()
		},
	);
	let config_path = store.config_dir().join(CONFIG_FILENAME);
	std::fs::remove_file(&config_path).unwrap();
	std::fs::create_dir(&config_path).unwrap();
	let server = ServerConfig {
		url: "https://example.com".to_owned(),
	};

	assert!(matches!(
		store.server_add(&server),
		Err(StoreError::Config(_))
	));
	assert!(store.config().servers.is_empty());
	assert!(store.has_zero_data_servers());
}

#[tokio::test(flavor = "multi_thread")]
async fn no_data_servers_fail_remote_operations_immediately() {
	let (_tmp, store) = new_test_store_with(
		"bb_remote_no_servers",
		BackbeatConfig {
			servers: Vec::new(),
			..BackbeatConfig::default()
		},
	);
	let asset_id = AssetId(Sha256::checksum_bytes(b"remote fixture"));

	assert!(matches!(
		store.server_has_asset(asset_id).await,
		Err(StoreError::Remote(RemoteError::NoServers))
	));
	assert!(matches!(
		store.server_download_asset(asset_id).await,
		Err(err) if matches!(&*err, StoreError::Remote(RemoteError::NoServers))
	));
}

#[tokio::test(flavor = "multi_thread")]
async fn has_asset_falls_back_to_next_server() {
	let first_hits = Arc::new(AtomicUsize::new(0));
	let second_hits = Arc::new(AtomicUsize::new(0));
	let first = spawn_head_server(404, Arc::clone(&first_hits)).await;
	let second = spawn_head_server(200, Arc::clone(&second_hits)).await;

	let (_tmp, store) = new_test_store_with(
		"bb_remote_servers",
		BackbeatConfig {
			servers: vec![ServerConfig { url: first }, ServerConfig { url: second }],
			..BackbeatConfig::default()
		},
	);

	let asset_id = AssetId(Sha256::checksum_bytes(b"remote fixture"));
	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("remote asset check")
	);
	assert_eq!(first_hits.load(Ordering::SeqCst), 1);
	assert_eq!(second_hits.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_server_remains_first_for_later_lookups() {
	let failed_hits = Arc::new(AtomicUsize::new(0));
	let live_hits = Arc::new(AtomicUsize::new(0));
	let failed = spawn_head_server(503, Arc::clone(&failed_hits)).await;
	let live = spawn_head_server(200, Arc::clone(&live_hits)).await;

	let (_tmp, store) = new_test_store_with(
		"bb_remote_config_order",
		BackbeatConfig {
			servers: vec![ServerConfig { url: failed }, ServerConfig { url: live }],
			..BackbeatConfig::default()
		},
	);

	let asset_id = AssetId(Sha256::checksum_bytes(b"config order fixture"));
	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("first lookup")
	);
	assert_eq!(failed_hits.load(Ordering::SeqCst), 1);
	assert_eq!(live_hits.load(Ordering::SeqCst), 1);

	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("second lookup")
	);
	assert_eq!(failed_hits.load(Ordering::SeqCst), 2);
	assert_eq!(live_hits.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_on_first_server_does_not_demote() {
	let first_hits = Arc::new(AtomicUsize::new(0));
	let second_hits = Arc::new(AtomicUsize::new(0));
	let first = spawn_head_server(404, Arc::clone(&first_hits)).await;
	let second = spawn_head_server(200, Arc::clone(&second_hits)).await;

	let (_tmp, store) = new_test_store_with(
		"bb_remote_no_demote_404",
		BackbeatConfig {
			servers: vec![ServerConfig { url: first }, ServerConfig { url: second }],
			..BackbeatConfig::default()
		},
	);

	let asset_id = AssetId(Sha256::checksum_bytes(b"missing fixture"));
	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("first lookup")
	);
	assert!(
		store
			.server_has_asset(asset_id)
			.await
			.expect("second lookup")
	);

	assert_eq!(first_hits.load(Ordering::SeqCst), 2);
	assert_eq!(second_hits.load(Ordering::SeqCst), 2);
}

async fn spawn_head_server(status: u16, hits: Arc<AtomicUsize>) -> String {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();

	tokio::spawn(async move {
		loop {
			let Ok((mut stream, _)) = listener.accept().await else {
				break;
			};
			let hits = Arc::clone(&hits);
			tokio::spawn(async move {
				let mut buf = [0u8; 1024];
				let _ = stream.read(&mut buf).await;
				hits.fetch_add(1, Ordering::SeqCst);
				let reason = match status {
					200 => "OK",
					404 => "Not Found",
					503 => "Service Unavailable",
					_ => "Unknown",
				};
				let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\n\r\n");
				let _ = stream.write_all(response.as_bytes()).await;
			});
		}
	});

	format!("http://{addr}")
}
