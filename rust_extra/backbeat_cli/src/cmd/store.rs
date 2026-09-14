use clap::Subcommand;

mod add;
mod check;
mod completely_destroy;
mod locate;
mod prune;
mod rm;

use crate::cmd::store::rm::RmCommand;

use self::check::CheckCommand;
use self::completely_destroy::CompletelyDestroyCommand;
use self::locate::LocateDbCommand;
use self::prune::{PruneCommand, PruneTarget};

/// Manage the local Backbeat store.
#[derive(Debug, Subcommand)]
pub enum StoreCommand {
	/// Check the integrity of the store.
	Check(CheckCommand),

	/// Print the path to `backbeat.db`.
	LocateDb(LocateDbCommand),

	/// Remove downloaded assets that are no longer referenced.
	AssetPrune(PruneCommand),

	/// Remove orphan asset files and stale partial downloads.
	DiskPrune(PruneCommand),

	/// Remove a bundle from your store.
	Rm(RmCommand),

	/// Permanently delete the store directory: database and assets.
	#[command(name = "completely-destroy-i-am-sure")]
	#[command(hide = true)]
	CompletelyDestroy(CompletelyDestroyCommand),
}

impl StoreCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		match self {
			Self::Check(cmd) => cmd.run().await,
			Self::LocateDb(cmd) => cmd.run().await,
			Self::AssetPrune(cmd) => cmd.run(PruneTarget::Asset).await,
			Self::DiskPrune(cmd) => cmd.run(PruneTarget::Disk).await,
			Self::CompletelyDestroy(cmd) => cmd.run().await,
			Self::Rm(cmd) => cmd.run().await,
		}
	}
}
