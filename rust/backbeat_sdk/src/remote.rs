use backbeat_server_client::{BackbeatServerClient, RemoteError};
use backbeat_store_config::ServerConfig;
use url::Url;

pub(crate) fn do_not_try_another_server(err: &RemoteError) -> bool {
	match err {
		RemoteError::NoServers => true,
		RemoteError::Cancelled => true,

		RemoteError::InvalidUrl(_) => false,
		RemoteError::Network(_) => false,
		RemoteError::Json(_) => false,
		RemoteError::NotFound => false,
		RemoteError::UnknownAlgorithm => false,
		RemoteError::Server(_) => false,
		RemoteError::UnexpectedStatus(_) => false,
	}
}

pub(crate) fn create_many_clients(servers: &[ServerConfig]) -> Vec<BackbeatServerClient> {
	let mut clients = Vec::with_capacity(servers.len());
	for server_config in servers {
		let url = match Url::parse(&server_config.url) {
			Ok(v) => v,
			Err(_err) => {
				tracing::warn!(url=%server_config.url, "Invalid URL in your config! Not using this remote.");
				continue;
			}
		};

		clients.push(BackbeatServerClient::new(&url));
	}

	clients
}
