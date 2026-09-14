use crate::store_util;
use anyhow::Context;
use backbeat_core::BundleId;
use clap::Args;

#[derive(Debug, Args)]
pub struct RmCommand {
	/// Bundle to remove.
	pub id: BundleId,
}

impl RmCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;

		store
			.bundle_rm(self.id)
			.context("failed to remove charts")?;

		Ok(())
	}
}
