#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![allow(unreachable_pub)]
//! Backbeat Data Server HTTP implementation.

mod asset_serve;
mod endpoints;
mod error;
mod error_response;
mod router;
mod state;

pub use asset_serve::{AssetServeResponse, asset_mime_type, serve_asset};
pub use error::ServeError;
pub use router::router;
pub use state::{AppState, ServerInfo};

use axum::Router;
use backbeat_sdk::Backbeat;
use tokio::net::TcpListener;

async fn shutdown_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c()
			.await
			.expect("failed to install Ctrl+C handler");
	};

	#[cfg(unix)]
	let terminate = async {
		tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		() = ctrl_c => {},
		() = terminate => {},
	}
}

/// Run the BDS HTTP server.
pub async fn serve(store: Backbeat, addr: std::net::SocketAddr) -> Result<(), ServeError> {
	let root = store.store_dir();
	tracing::info!(path = %root.display(), "serving data from store");

	match store.stats() {
		Ok(stats) => {
			let charts = stats.charts;
			let assets = stats.asset_count;
			match (charts, assets) {
				(0, 0) => tracing::info!("store is empty"),
				_ => tracing::info!(charts, assets, "serving store content"),
			}
		}
		Err(err) => tracing::warn!(error = %err, "failed to read store inventory"),
	}

	let state = AppState::new(store, addr).map_err(ServeError::Config)?;

	let socket = state.addr;
	let listener = TcpListener::bind(socket).await?;
	let app: Router = router(state);

	tracing::info!("Backbeat data server listening on http://{socket}");
	axum::serve(listener, app)
		.with_graceful_shutdown(shutdown_signal())
		.await?;

	Ok(())
}
