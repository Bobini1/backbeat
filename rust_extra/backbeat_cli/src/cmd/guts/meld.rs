use std::path::PathBuf;

use anyhow::Context;
use backbeat_core::BackbeatFile;
use clap::Args;

/// Combine compatible fractured bundles into one .bb file.
#[derive(Debug, Args)]
pub struct MeldCommand {
	/// Path where the combined .bb file will be written.
	pub output: PathBuf,

	/// Fractured .bb files to combine.
	#[arg(required = true, num_args = 1..)]
	pub inputs: Vec<PathBuf>,
}

impl MeldCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let bundles = self
			.inputs
			.iter()
			.map(|path| {
				BackbeatFile::from_file(path).with_context(|| format!("failed to read {path:?}"))
			})
			.collect::<anyhow::Result<Vec<_>>>()?;

		let output_bundle = backbeat_packager::meld(&bundles).context("failed to meld bundles")?;

		let json = output_bundle.to_json();

		fs_err::write(&self.output, json)
			.with_context(|| format!("failed to write {:?}", self.output))?;
		println!("wrote {}", self.output.display());

		Ok(())
	}
}
