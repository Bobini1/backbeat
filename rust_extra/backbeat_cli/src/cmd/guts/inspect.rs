use std::path::PathBuf;

use anyhow::Context;
use backbeat_core::BackbeatFile;
use backbeat_inspector::inspect_bundle;
use clap::{Args, ValueEnum};

#[derive(Debug, Args)]
pub struct InspectCommand {
	pub path: PathBuf,

	#[arg(short, long, value_enum, default_value_t = InspectOutput::Human)]
	pub output: InspectOutput,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InspectOutput {
	Human,
	Json,
}

impl InspectCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let bundles = if self.path.extension().is_some_and(|ext| ext == "bb") {
			vec![
				BackbeatFile::from_file(&self.path)
					.with_context(|| format!("failed to read .bb {:?}", self.path))?,
			]
		} else {
			backbeat_packager::package(&self.path)
				.with_context(|| format!("failed to package {:?}", self.path))?
		};

		for inspection in bundles.iter().map(inspect_bundle) {
			let inspection = inspection?;
			match self.output {
				InspectOutput::Human => {
					println!("[{}]", inspection.description);
					println!("  gamemode: {}", inspection.gamemode);
					println!("  sha256: {}", inspection.chart_sha256);
					for id in inspection.chart_ids {
						println!("  {}: {}", id.alg, id.val);
					}
				}
				InspectOutput::Json => {
					serde_json::to_writer_pretty(std::io::stdout(), &inspection)?;
					println!();
				}
			}
		}

		Ok(())
	}
}
