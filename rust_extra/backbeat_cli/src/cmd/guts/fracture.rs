use std::path::PathBuf;

use anyhow::Context;
use clap::Args;

use super::output::resolve_output_dir;

/// Split a multi-chart source file into one raw chart file per playable chart.
///
/// Writes uncompressed chart bytes (not `.bb`). Only `.sm` and `.ssc` are
/// supported. Outputs are written as `{stem}.1.{ext}`, `{stem}.2.{ext}`, …
/// next to the source, or into `--output-dir` when set.
#[derive(Debug, Args)]
pub struct FractureCommand {
	/// Path to the source chart file (`.sm` or `.ssc`).
	pub file: PathBuf,

	/// Directory to write fractured charts into. Defaults to the source file's directory.
	#[arg(short = 'o', long = "output-dir")]
	pub output_dir: Option<PathBuf>,
}

impl FractureCommand {
	pub fn run(self) -> anyhow::Result<()> {
		anyhow::ensure!(
			backbeat_packager::should_be_packaged(&self.file),
			"unrecognised chart format: {}",
			self.file.display()
		);
		let charts = backbeat_packager::package(&self.file)
			.with_context(|| format!("failed to fracture {}", self.file.display()))?;

		let out_dir = resolve_output_dir(&self.file, self.output_dir.as_deref())?;
		let stem = self
			.file
			.file_stem()
			.map(|stem| stem.to_string_lossy())
			.unwrap_or_default();
		let width = charts.len().to_string().len();

		let extension = self
			.file
			.extension()
			.and_then(|ext| ext.to_str())
			.unwrap_or_default();
		for (index, chart) in charts.iter().enumerate() {
			let filename = format!("{stem}.{:0width$}.{}", index + 1, extension, width = width);
			let out = out_dir.join(filename);
			fs_err::write(&out, chart.chart.decompress())
				.with_context(|| format!("failed to write {}", out.display()))?;
			println!("wrote {}", out.display());
		}

		Ok(())
	}
}
