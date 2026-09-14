use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, ValueEnum};

#[derive(Debug, Args)]
pub struct MediaInfoCommand {
	pub path: PathBuf,

	#[arg(short, long, value_enum, default_value_t = MediaInfoOutput::Human)]
	pub output: MediaInfoOutput,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MediaInfoOutput {
	Human,
	Json,
}

impl MediaInfoCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let info = media_file_info::extract(&self.path)
			.with_context(|| format!("failed to inspect media file {:?}", self.path))?;

		match self.output {
			MediaInfoOutput::Human => {
				println!("{}", self.path.display());
				println!("  kind: {}", info.specific.as_str());
				println!("  mime type: {}", info.common.mime_type);
				println!("  size: {} bytes", info.common.size_bytes);
				if let Some(format_name) = info.common.format_name {
					println!("  format: {format_name}");
				}
				println!("  details: {:#?}", info.specific);
			}
			MediaInfoOutput::Json => {
				serde_json::to_writer_pretty(std::io::stdout(), &info)?;
				println!();
			}
		}

		Ok(())
	}
}
