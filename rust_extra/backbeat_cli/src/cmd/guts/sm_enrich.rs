use std::path::PathBuf;

use anyhow::Context;
use clap::Args;

#[derive(Debug, Args)]
pub struct SmEnrichCommand {
	/// Path to the unfractured `.sm`, `.ssc`, or `.dwi` file to update.
	pub file: PathBuf,

	/// Recursively enrich every `.sm`, `.ssc`, or `.dwi` file below the given directory.
	#[arg(short, long)]
	pub recursive: bool,
}

impl SmEnrichCommand {
	pub fn run(self) -> anyhow::Result<()> {
		if self.recursive {
			return enrich_recursive(&self.file);
		}

		anyhow::ensure!(
			self.file.extension().is_some_and(|extension| {
				extension.eq_ignore_ascii_case("sm")
					|| extension.eq_ignore_ascii_case("ssc")
					|| extension.eq_ignore_ascii_case("dwi")
			}),
			"bkb guts sm-enrich only accepts .sm, .ssc, or .dwi files"
		);
		tracing::info!(file = %self.file.display(), "enriching chart file");
		backbeat_packager::sm_enrich::enrich_sm_file(&self.file)
			.with_context(|| format!("failed to enrich {}", self.file.display()))?;
		tracing::info!(file = %self.file.display(), "enriched chart file");
		Ok(())
	}
}

fn enrich_recursive(root: &std::path::Path) -> anyhow::Result<()> {
	anyhow::ensure!(
		root.is_dir(),
		"--recursive requires a directory, got: {}",
		root.display()
	);
	tracing::info!(directory = %root.display(), "enriching chart files recursively");

	let mut enriched = 0;
	for entry in walkdir::WalkDir::new(root)
		.follow_links(false)
		.sort_by_file_name()
		.into_iter()
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().is_file())
	{
		let path = entry.path();
		if !path.extension().is_some_and(|extension| {
			extension.eq_ignore_ascii_case("sm")
				|| extension.eq_ignore_ascii_case("ssc")
				|| extension.eq_ignore_ascii_case("dwi")
		}) {
			continue;
		}

		tracing::info!(file = %path.display(), "enriching chart file");
		backbeat_packager::sm_enrich::enrich_sm_file(path)
			.with_context(|| format!("failed to enrich {}", path.display()))?;
		enriched += 1;
	}

	tracing::info!(directory = %root.display(), files = enriched, "finished enriching chart files");
	Ok(())
}
