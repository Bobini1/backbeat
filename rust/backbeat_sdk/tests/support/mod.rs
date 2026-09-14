#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use backbeat_sdk::Backbeat;
use backbeat_store_config::BackbeatConfig;
use tempfile::TempDir;

use tokio::sync::Mutex;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// Duplicated from the crate's test utilities because integration tests do not compile the library with `cfg(test)`.
pub fn new_test_store(tmpdir_name: &str) -> (TempDir, Backbeat) {
	new_test_store_with(tmpdir_name, BackbeatConfig::default())
}

pub fn new_test_store_with(tmpdir_name: &str, mut config: BackbeatConfig) -> (TempDir, Backbeat) {
	let temp = tempfile::Builder::new()
		.prefix(tmpdir_name)
		.tempdir()
		.expect("create test store temp directory");
	let config_dir = temp.path().join("config");
	config.store.path = temp.path().join("data");
	config
		.write_to_dir(&config_dir)
		.expect("write test store config");
	let store = Backbeat::open_with_overridden_config_dir(&config_dir)
		.expect("open test store from temporary config");
	(temp, store)
}

pub async fn spawn_collection_server(routes: Vec<(String, String)>) -> String {
	let routes = Arc::new(routes.into_iter().collect::<HashMap<_, _>>());
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let address = listener.local_addr().unwrap();
	tokio::spawn(async move {
		while let Ok((mut stream, _)) = listener.accept().await {
			let routes = Arc::clone(&routes);
			tokio::spawn(async move {
				let mut buffer = [0u8; 4096];
				let size = stream.read(&mut buffer).await.unwrap_or(0);
				let request = String::from_utf8_lossy(&buffer[..size]);
				let path = request
					.lines()
					.next()
					.and_then(|line| line.split_whitespace().nth(1))
					.unwrap_or("");
				let (status, body) = routes
					.get(path)
					.map(|body| ("200 OK", body.as_bytes()))
					.unwrap_or(("404 Not Found", &[]));
				let response = format!(
					"HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
					body.len()
				);
				let _ = stream.write_all(response.as_bytes()).await;
				let _ = stream.write_all(body).await;
			});
		}
	});
	format!("http://{address}")
}

#[allow(dead_code)]
pub async fn spawn_mutable_collection_server(
	routes: Vec<(String, String)>,
) -> (String, Arc<Mutex<HashMap<String, String>>>) {
	let routes = Arc::new(Mutex::new(routes.into_iter().collect::<HashMap<_, _>>()));
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let address = listener.local_addr().unwrap();
	let shared_routes = Arc::clone(&routes);
	tokio::spawn(async move {
		while let Ok((mut stream, _)) = listener.accept().await {
			let routes = Arc::clone(&shared_routes);
			tokio::spawn(async move {
				let mut buffer = [0u8; 4096];
				let size = stream.read(&mut buffer).await.unwrap_or(0);
				let request = String::from_utf8_lossy(&buffer[..size]);
				let path = request
					.lines()
					.next()
					.and_then(|line| line.split_whitespace().nth(1))
					.unwrap_or("");
				let routes = routes.lock().await;
				let (status, body) = routes
					.get(path)
					.map(|body| ("200 OK", body.as_bytes()))
					.unwrap_or(("404 Not Found", &[]));
				let response = format!(
					"HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
					body.len()
				);
				let _ = stream.write_all(response.as_bytes()).await;
				let _ = stream.write_all(body).await;
			});
		}
	});
	(format!("http://{address}"), routes)
}

pub async fn spawn_http_server(
	routes: Vec<(String, Vec<u8>)>,
	include_content_length: bool,
) -> String {
	spawn_delayed_http_server(routes, include_content_length, Duration::ZERO).await
}

pub async fn spawn_delayed_http_server(
	routes: Vec<(String, Vec<u8>)>,
	include_content_length: bool,
	response_delay: Duration,
) -> String {
	let routes = Arc::new(routes.into_iter().collect::<HashMap<_, _>>());
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let address = listener.local_addr().unwrap();
	tokio::spawn(async move {
		while let Ok((mut stream, _)) = listener.accept().await {
			let routes = Arc::clone(&routes);
			tokio::spawn(async move {
				let mut buffer = [0u8; 4096];
				let size = stream.read(&mut buffer).await.unwrap_or(0);
				let request = String::from_utf8_lossy(&buffer[..size]);
				let path = request
					.lines()
					.next()
					.and_then(|line| line.split_whitespace().nth(1))
					.unwrap_or("");
				let (status, body) = routes
					.get(path)
					.map(|body| ("200 OK", body.as_slice()))
					.unwrap_or(("404 Not Found", &[]));
				let content_length = if include_content_length {
					format!("Content-Length: {}\r\n", body.len())
				} else {
					String::new()
				};
				let response = format!(
					"HTTP/1.1 {status}\r\n{content_length}Content-Type: application/octet-stream\r\nConnection: close\r\n\r\n"
				);
				tokio::time::sleep(response_delay).await;
				let _ = stream.write_all(response.as_bytes()).await;
				let _ = stream.write_all(body).await;
			});
		}
	});
	format!("http://{address}")
}
