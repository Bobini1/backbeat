//! Manage `[[server]]` entries in `backbeat.toml` -- the remote Backbeat
//! Content Servers this store pulls charts/assets from.
//!
//! Adding or removing a remote updates both the on-disk config and the live
//! [`Backbeat`] client list, so downloads pick up the change without restarting
//! the app.

use std::time::Instant;

use backbeat_store_config::{BackbeatConfig, BackbeatServerInfo, ServerConfig, default_config_dir};
use serde::Serialize;
use tauri::State;
use url::Url;

use crate::AppState;

#[tauri::command]
#[tracing::instrument]
pub fn list_remotes() -> Result<Vec<ServerConfig>, String> {
	let started = Instant::now();
	let remotes = load_config()?.servers;

	tracing::debug!(count = remotes.len(), "listed configured remotes");
	tracing::debug!(
		count = remotes.len(),
		elapsed_ms = started.elapsed().as_millis(),
		"startup timing: list_remotes ready"
	);
	Ok(remotes)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn add_remote(state: State<'_, AppState>, url: String) -> Result<(), String> {
	let resolved = correct_server_url(&url).await?;
	let entry = ServerConfig {
		url: resolved.url.clone(),
	};
	tracing::info!(
		input_url = %url,
		server_url = %resolved.url,
		server_name = resolved.info.name.as_str(),
		"remote passed /backbeat/info check"
	);

	state.store.get()?.server_add(&entry).map_err(|err| {
		tracing::error!(?err, input_url = %url, server_url = %resolved.url, "failed to persist new remote");
		err.to_string()
	})?;

	tracing::info!(input_url = %url, server_url = %resolved.url, "remote added");
	Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remove_remote(state: State<'_, AppState>, url: String) -> Result<(), String> {
	let resolved_url = match correct_server_url(&url).await {
		Ok(resolved) => resolved.url,
		Err(err) => {
			tracing::warn!(%url, %err, "could not correct remote URL before removal");
			url.clone()
		}
	};
	let entry = ServerConfig {
		url: resolved_url.clone(),
	};

	state.store.get()?.server_rm(&entry).map_err(|err| {
		tracing::error!(?err, input_url = %url, server_url = %resolved_url, "failed to persist remote removal");
		err.to_string()
	})?;

	tracing::info!(input_url = %url, server_url = %resolved_url, "remote removed");
	Ok(())
}

#[derive(Serialize)]
pub struct PingResult {
	pub up: bool,
	pub name: Option<String>,
	pub contact: Option<String>,
	pub latency_ms: u64,
	pub error: Option<String>,
}

/// Hit `{url}/backbeat/info` directly: cheap enough to double as a liveness
/// check, and it also returns the server's display name/contact for free.
#[tauri::command]
#[tracing::instrument]
#[allow(clippy::cast_possible_truncation)]
pub async fn ping_remote(url: String) -> PingResult {
	let started = Instant::now();
	let elapsed_ms = || started.elapsed().as_millis() as u64;

	tracing::debug!(%url, "pinging remote");

	match fetch_server_info(&url).await {
		Ok(info) => {
			tracing::info!(
				%url,
				server_name = info.name.as_str(),
				latency_ms = elapsed_ms(),
				"remote ping succeeded"
			);
			PingResult {
				up: true,
				name: Some(info.name),
				contact: info.contact,
				latency_ms: elapsed_ms(),
				error: None,
			}
		}
		Err(err) => {
			tracing::warn!(%url, %err, latency_ms = elapsed_ms(), "remote ping failed");
			PingResult {
				up: false,
				name: None,
				contact: None,
				latency_ms: elapsed_ms(),
				error: Some(err),
			}
		}
	}
}

/// Fetch `{url}/backbeat/info` and decode it as a [`ServerInfo`].
pub(crate) async fn fetch_server_info(url: &str) -> Result<BackbeatServerInfo, String> {
	let url = Url::parse(url).map_err(|_| format!("Invalid URL {url}"))?;
	let client = backbeat_server_client::BackbeatServerClient::new(&url);

	client
		.get_info()
		.await
		.map_err(|err| format!("invalid /backbeat/info response: {err}"))
}

struct ResolvedServer {
	url: String,
	info: BackbeatServerInfo,
}

async fn correct_server_url(input_url: &str) -> Result<ResolvedServer, String> {
	let mut candidates = vec![input_url.to_owned()];
	let origin = server_origin(input_url).map_err(|err| {
		tracing::debug!(%input_url, %err, "invalid server URL");
		"This URL isn't a Backbeat server.".to_owned()
	})?;
	if origin != input_url.trim_end_matches('/') {
		candidates.push(origin);
	}

	let mut errors = Vec::new();
	for candidate in candidates {
		match fetch_server_info(&candidate).await {
			Ok(info) => {
				return Ok(ResolvedServer {
					url: candidate,
					info,
				});
			}
			Err(err) => errors.push(format!("{candidate}/backbeat/info: {err}")),
		}

		match advertised_server(&candidate).await {
			Ok(Some(url)) => match fetch_server_info(&url).await {
				Ok(info) => return Ok(ResolvedServer { url, info }),
				Err(err) => errors.push(format!("{candidate}: advertised server failed: {err}")),
			},
			Ok(None) => errors.push(format!("{candidate}: no X-Backbeat-Server header")),
			Err(err) => errors.push(format!("{candidate}: {err}")),
		}
	}

	tracing::debug!(%input_url, errors = %errors.join("; "), "server URL correction failed");
	Err("This URL isn't a Backbeat server.".to_owned())
}

async fn advertised_server(url: &str) -> Result<Option<String>, String> {
	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_secs(15))
		.build()
		.map_err(|err| format!("failed to build HTTP client: {err}"))?;
	let response = client
		.get(url)
		.send()
		.await
		.map_err(|err| format!("failed to connect: {err}"))?;

	let Some(server) = response.headers().get("X-Backbeat-Server") else {
		return Ok(None);
	};
	let server = server
		.to_str()
		.map_err(|err| format!("invalid X-Backbeat-Server header: {err}"))?;
	let parsed = Url::parse(server)
		.map_err(|err| format!("invalid X-Backbeat-Server URL {server:?}: {err}"))?;
	Ok(Some(parsed.as_str().trim_end_matches('/').to_owned()))
}

fn server_origin(url: &str) -> Result<String, String> {
	let mut parsed = Url::parse(url).map_err(|err| format!("invalid server URL {url:?}: {err}"))?;
	parsed.set_path("");
	parsed.set_query(None);
	parsed.set_fragment(None);
	Ok(parsed.as_str().trim_end_matches('/').to_owned())
}

fn load_config() -> Result<BackbeatConfig, String> {
	BackbeatConfig::load_with_overridden_dir(&default_config_dir()).map_err(|err| {
		tracing::error!(?err, "failed to load backbeat config");
		err.to_string()
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use axum::{Json, Router, http::header, routing::get};

	async fn start_test_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let task = tokio::spawn(async move {
			axum::serve(listener, app).await.unwrap();
		});
		(format!("http://{address}"), task)
	}

	fn server_info() -> BackbeatServerInfo {
		BackbeatServerInfo {
			name: "Test server".into(),
			contact: None,
		}
	}

	#[test]
	fn server_origin_removes_path_query_and_fragment() {
		assert_eq!(
			server_origin("https://data.backbeat.ac/export/v1?key=value#chart").unwrap(),
			"https://data.backbeat.ac"
		);
	}

	#[tokio::test]
	async fn correct_server_url_retries_without_a_path() {
		let app = Router::new().route("/backbeat/info", get(|| async { Json(server_info()) }));
		let (url, task) = start_test_server(app).await;

		let resolved = correct_server_url(&format!("{url}/unused/path"))
			.await
			.unwrap();
		assert_eq!(resolved.url, url);

		task.abort();
	}

	#[tokio::test]
	async fn correct_server_url_uses_advertised_server_header() {
		let target = Router::new().route("/backbeat/info", get(|| async { Json(server_info()) }));
		let (target_url, target_task) = start_test_server(target).await;
		let advertised_url = target_url.clone();
		let source = Router::new().route(
			"/",
			get(move || {
				let advertised_url = advertised_url.clone();
				async move {
					(
						[(
							header::HeaderName::from_static("x-backbeat-server"),
							advertised_url,
						)],
						(),
					)
				}
			}),
		);
		let (source_url, source_task) = start_test_server(source).await;

		let resolved = correct_server_url(&source_url).await.unwrap();
		assert_eq!(resolved.url, target_url);

		source_task.abort();
		target_task.abort();
	}
}
