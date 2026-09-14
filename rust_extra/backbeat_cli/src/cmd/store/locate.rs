use backbeat_sdk::Backbeat;
use clap::Args;

use crate::store_util;

/// Print the path to `backbeat.db`.
#[derive(Debug, Args)]
pub struct LocateDbCommand {}

impl LocateDbCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;
		println!(
			"{}",
			store.store_dir().join(Backbeat::DB_FILENAME).display()
		);
		Ok(())
	}
}
