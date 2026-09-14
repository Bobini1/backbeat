use anyhow::Context;
use clap::Args;

use super::completely_destroy::confirm;
use crate::{fmt, store_util, style};

#[derive(Clone, Copy)]
pub(super) enum PruneTarget {
	Asset,
	Disk,
}

/// Remove unused data from the Backbeat store.
#[derive(Debug, Args)]
pub struct PruneCommand {
	/// Show what would be removed without actually deleting anything.
	#[arg(long)]
	pub dry_run: bool,

	/// Skip the confirmation prompt and prune immediately.
	#[arg(short, long)]
	pub yes: bool,
}

impl PruneCommand {
	pub(super) async fn run(self, target: PruneTarget) -> anyhow::Result<()> {
		let store = store_util::open()?;
		let (label, noun, count) = match target {
			PruneTarget::Asset => (
				"Unused assets",
				"asset",
				store
					.asset_prune(true)
					.context("failed to scan for unused assets")?,
			),
			PruneTarget::Disk => (
				"Orphan files",
				"file",
				store
					.disk_prune(true)
					.context("failed to scan for filesystem orphans")?,
			),
		};

		if count == 0 {
			println!("Nothing to prune.");
			return Ok(());
		}

		println!("{label}: {}", fmt::count(count));

		if self.dry_run {
			println!("Dry run; nothing was removed.");
			return Ok(());
		}

		if !self.yes && !confirm("Prune these items?")? {
			println!("Aborted.");
			return Ok(());
		}

		let removed = match target {
			PruneTarget::Asset => store
				.asset_prune(false)
				.context("failed to prune unused assets")?,
			PruneTarget::Disk => store
				.disk_prune(false)
				.context("failed to prune filesystem orphans")?,
		};

		println!(
			"{} Pruned {} {noun}{}.",
			style::check_mark(),
			fmt::count(removed),
			if removed == 1 { "" } else { "s" },
		);
		Ok(())
	}
}
