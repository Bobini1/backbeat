use crate::{store_util, style};
use anyhow::Context as _;
use backbeat_core::{BundleId, ChartId};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DownloadCommand {
	/// Download a chart and all of its assets.
	Chart { id: ChartId },
	/// Download a bundle and all of its assets.
	Bundle { id: BundleId },
}

impl DownloadCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;

		match self {
			Self::Chart { id } => {
				let spinner = style::new_spinner(format!("Downloading chart {id}…"));
				store
					.server_download_chart(&id)
					.await
					.context("failed to download chart")?;
				spinner.finish_and_clear();
				println!("{} Downloaded chart successfully", style::check_mark());
			}
			Self::Bundle { id } => {
				let spinner = style::new_spinner(format!("Downloading bundle {id}…"));
				store
					.server_download_bundle(id)
					.await
					.context("failed to download bundle")?;
				spinner.finish_and_clear();
				println!("{} Downloaded bundle successfully", style::check_mark());
			}
		}

		Ok(())
	}
}
