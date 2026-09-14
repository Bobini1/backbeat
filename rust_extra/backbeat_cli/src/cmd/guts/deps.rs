use std::path::PathBuf;

use anyhow::Context;
use backbeat_core::AssetPath;
use clap::Args;

/// List the asset dependencies of a source chart file.
#[derive(Debug, Args)]
pub struct DepsCommand {
	/// Path to the source chart file (e.g. a .bms, .ksh, .sm file).
	pub file: PathBuf,

	/// Fail if any referenced dependency is missing from the filesystem.
	#[arg(long)]
	pub strict: bool,
}

impl DepsCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let (bbs, mut missing) = backbeat_packager::package_with_missing_assets(&self.file)
			.with_context(|| format!("failed to package {:?}", self.file))?;
		let chart = bbs
			.first()
			.with_context(|| format!("no charts packaged from {:?}", self.file))?;

		if self.strict {
			super::require_no_missing_dependencies(missing)?;
		} else {
			missing.sort_unstable();
			missing.dedup();
			for path in missing {
				eprintln!("warning: referenced dependency does not exist: {path}",);
			}
		}

		let mut paths: Vec<&AssetPath> = chart.assets.keys().collect();
		paths.sort_unstable();

		if paths.is_empty() {
			println!("(no dependencies)");
		} else {
			for path in &paths {
				println!("{path}");
			}
		}

		Ok(())
	}
}
