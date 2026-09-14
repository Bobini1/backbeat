use std::net::SocketAddr;
use std::sync::Arc;

use backbeat_sdk::Backbeat;
use backbeat_store_config::{BackbeatServerInfo, ConfigError};
use serde::Serialize;

/// Metadata served at `/backbeat/info`.
#[derive(Debug, Serialize, Clone)]
pub struct ServerInfo {
	pub name: String,
	pub contact: Option<String>,
}

impl Default for ServerInfo {
	fn default() -> Self {
		Self {
			name: "Unnamed Server".to_string(),
			contact: None,
		}
	}
}

impl ServerInfo {
	fn from_publish(config: &BackbeatServerInfo) -> Self {
		Self {
			name: config.name.clone(),
			contact: config.contact.clone(),
		}
	}
}

#[derive(Clone)]
pub struct AppState {
	pub store: Arc<Backbeat>,
	pub info: Option<ServerInfo>,
	pub addr: SocketAddr,
}

impl AppState {
	pub fn new(store: Backbeat, addr: SocketAddr) -> Result<Self, ConfigError> {
		let config = store.config();
		let publish = config.info.as_ref();
		let info = publish.map(ServerInfo::from_publish);

		Ok(Self {
			store: Arc::new(store),
			info,
			addr,
		})
	}
}
