use anyhow::Context;
use clap::Args;
use std::net::{IpAddr, SocketAddr};

use crate::store_util;

/// `bkb start-data-server`
#[derive(Debug, Args)]
pub struct StartDataServerCommand {
	/// Address to bind (default: 0.0.0.0).
	#[arg(long, default_value = "0.0.0.0")]
	pub bind: IpAddr,

	/// Port to listen on (default: 8080).
	#[arg(long, default_value_t = 8080)]
	pub port: u16,
}

impl StartDataServerCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;
		let addr = SocketAddr::new(self.bind, self.port);

		backbeat_server::serve(store, addr)
			.await
			.context("data server failed")
	}
}
