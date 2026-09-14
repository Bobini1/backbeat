use anyhow::Context;
use backbeat_sdk::Backbeat;
use backbeat_store_config::{CONFIG_FILENAME, default_config_dir};
use clap::Args;

/// Ensure the global Backbeat store is set up.
#[derive(Debug, Args)]
pub struct InitCommand {}

impl InitCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let config_dir = default_config_dir();
		let store = Backbeat::open().context("failed to initialise Backbeat store")?;

		println!("config:  {}", config_dir.join(CONFIG_FILENAME).display());
		println!("data:    {}", store.store_dir().display());

		Ok(())
	}
}
